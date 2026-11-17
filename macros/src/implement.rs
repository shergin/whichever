//! `implement!`: the forwarding impl of one trait for one subject.
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_quote_spanned, Error, Generics, Ident, Path, Result};

use crate::agreement::Agreement;
use crate::combined;
use crate::items::ItemWriter;
use crate::options::{Override, TraitOptions};
use crate::partial::PartialBlock;
use crate::protocol::ImplementInput;
use crate::signatures::Signatures;
use crate::site::{at, Site};
use crate::spelling;
use crate::subject::{Entries, Subject, Target};
use crate::substitution::Substitution;
use crate::words::Words;

pub(crate) fn expand_implement(input: ImplementInput) -> Result<TokenStream2> {
    let ImplementInput {
        signatures,
        subject,
        subject_remote,
        path: spec_path,
        mut options,
        partial,
    } = input;
    let item_trait = &signatures.item;
    let implemented = signatures.implemented_path(&spec_path);
    let trait_path = &implemented;
    let trait_ident = &trait_path
        .segments
        .last()
        .expect("a path has at least one segment")
        .ident;
    let words = subject.words();
    let block = partial
        .map(|block| PartialBlock::read(block, words))
        .transpose()?;
    // An associated type written in the block is an override like the
    // `Name = Type` option.
    for (name, ty) in block.iter().flat_map(|block| &block.declared_types) {
        options.overrides.push(Override::of_type(name.clone(), ty));
    }
    // Patterns and values name the subject by its real path when it is a
    // restatement, and by its own name otherwise.
    let subject_path = subject_remote
        .clone()
        .unwrap_or_else(|| Path::from(subject.ident().clone()));
    // Every token written here carries the subject's span: the call site is
    // the trait's attribute, which may be in another crate.
    let site = Site::at(subject.ident().span());
    // The impl's header: the partial block's when there is one, or the
    // subject's own, where an `unsafe trait` is implemented by an `unsafe
    // impl`.
    let (generics, self_ty, impl_attrs, unsafety) = match &block {
        Some(block) => (
            &block.generics,
            block.self_ty.clone(),
            block.attrs.as_slice(),
            block.unsafety,
        ),
        None => {
            let (_, ty_generics, _) = subject.generics().split_for_impl();
            (
                subject.generics(),
                parse_quote_spanned!(*site => #subject_path #ty_generics),
                &[][..],
                item_trait.unsafety,
            )
        }
    };
    let self_ty = &self_ty;
    let user_items = block
        .as_ref()
        .map(|block| block.user_items.as_slice())
        .unwrap_or_default();
    let derived = block
        .is_none()
        .then(|| quote_spanned!(*site => #[automatically_derived]));
    // The partial impl block a message points at, spelled with the trait as
    // the subject names it and the subject's own generics.
    let block_header = {
        let (impl_generics, ty_generics, _) = subject.generics().split_for_impl();
        spelling::spelled(
            &quote_spanned!(*site => impl #impl_generics #spec_path for #subject_path #ty_generics),
        )
    };
    let mut entries = subject.read()?.for_trait(&spec_path)?;
    // A partial block's header may name the subject's parameters as it
    // likes, `impl<U> Trait for Kind<U>` over `enum Kind<T>`; the payloads
    // are spelled with the names the subject declared, and follow.
    if let Some(block) = &block {
        if let Some(mut renaming) = Substitution::of_subject(subject.generics(), &block.self_ty) {
            entries.rename(&mut renaming);
        }
    }
    let route_errors = entries.route(&options.routes);
    let variants = entries.targets();
    // The bounds a hand writes: a payload that is a type parameter of the
    // subject gets `Param: Trait` on the impl, which the arms need and
    // nothing else supplies. A partial block's header is the user's and is
    // written as it is.
    let generics = match block {
        Some(_) => generics.clone(),
        None => with_payload_bounds(generics, &variants, subject.generics(), trait_path),
    };
    let generics = &generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    if variants.is_empty() {
        if entries.is_empty() {
            return Err(Error::new_spanned(
                &subject,
                "a dispatch enum needs at least one variant",
            ));
        }
        // Every variant was rejected, each with its own sentence; an impl
        // with nothing to forward to would only add rustc's errors to them.
        return Ok(match &block {
            Some(_) => entries.shape_errors(None),
            None => TokenStream2::new(),
        });
    }
    // The associated items come from the first variant that is always
    // compiled in and whose target has a type to name; one under `#[cfg]`
    // may not be there to supply them, and one reached through a field or
    // a method has no type to project from.
    let first = variants
        .iter()
        .find(|target| !target.entry.is_conditional() && target.target_type().is_some())
        .copied();
    let of = Impl {
        subject: &subject,
        subject_path: &subject_path,
        trait_path,
        trait_ident,
        signatures: &signatures,
        options: &options,
        entries: &entries,
        block: block.as_ref(),
        block_header: &block_header,
        supplier: first,
        site,
    };
    // What the impl names around a method's signature, in its header and in
    // its arms: a method's own generic parameter keeps clear of these.
    let mut substitution = Substitution::of_trait(item_trait, trait_path, generics)?;
    substitution.names(generics.to_token_stream());
    substitution.names(generics.where_clause.to_token_stream());
    substitution.names(self_ty.to_token_stream());
    substitution.names(subject_path.to_token_stream());
    for target in &variants {
        substitution.names(target.payload.to_token_stream());
    }
    for fixed in &options.overrides {
        substitution.names(fixed.to_token_stream());
    }
    for (_, result) in &options.mismatch {
        substitution.names(result.to_token_stream());
    }
    // Every item of the trait is planned and written in turn; every item
    // that cannot be forwarded is reported, not only the first.
    let mut writer = ItemWriter::new(&of, substitution);
    for item in &item_trait.items {
        writer.write(item);
    }
    let ItemWriter {
        items,
        agreed_types,
        agreed_consts,
        deprecated_items,
        plain_forwarding,
        errors,
        ..
    } = writer;
    // An option naming nothing in the trait, or a route through no variant,
    // is reported beside the impl, which is right without it; only an item
    // that cannot be written withholds the impl.
    let mut reported = route_errors;
    reported.extend(options.unknown_names(item_trait, trait_ident, words, block.is_none()));
    // An item that withholds the impl is reported with the errors that
    // would have stood beside it, so that none waits for the next build.
    if !errors.is_empty() {
        let mut errors = errors;
        errors.extend(reported);
        return Err(combined(errors).expect("at least one error"));
    }
    let reported = combined(reported).map(|error| error.to_compile_error());
    // The checks live outside the impl, where a `Self` in a payload type
    // has to be spelled out as the implementing type.
    let checks = first.map(|first| {
        let first_outside = first
            .target_type_outside(self_ty)
            .expect("the supplier is chosen among the targets with a type");
        let agreement = Agreement {
            variants: &variants,
            first,
            trait_path,
            first_payload: &first_outside,
            generics,
            self_type: self_ty,
            site,
            deprecated: deprecated_items,
        };
        let types = agreement.types(&agreed_types);
        let consts = agreement.consts(&agreed_consts);
        quote_spanned! { *site => #types #consts }
    });
    // A partial block is the only dispatch of its trait, so a variant that
    // cannot be forwarded to is reported here, under its own `cfg`, in the
    // builds in which some method forwards to every variant; a block whose
    // every method names its variants leaves such a variant to the bodies.
    let shape_errors = match &block {
        Some(_) if !plain_forwarding.is_never() => {
            entries.shape_errors(plain_forwarding.attribute(*site).as_ref())
        }
        _ => TokenStream2::new(),
    };
    Ok(quote_spanned! { *site =>
        #(#impl_attrs)*
        #derived
        #unsafety impl #impl_generics #trait_path for #self_ty #where_clause {
            #items
            #(#user_items)*
        }
        #checks
        #shape_errors
        #reported
    })
}

/// One trait's impl for one subject: what every item of it is written
/// against.
pub(crate) struct Impl<'a> {
    pub(crate) subject: &'a Subject,
    /// The subject's real path, for patterns and values: its own name, or
    /// the path a restatement names.
    pub(crate) subject_path: &'a Path,
    /// The trait as the impl names it: a restated trait's real path, or
    /// the path the subject named.
    pub(crate) trait_path: &'a Path,
    pub(crate) trait_ident: &'a Ident,
    /// The trait as carried, which says what it gives each method.
    pub(crate) signatures: &'a Signatures,
    pub(crate) options: &'a TraitOptions,
    /// The entries the trait is forwarded to.
    pub(crate) entries: &'a Entries<'a>,
    /// The partial block the impl is written around, when there is one;
    /// without one the impl is the subject's attribute's,
    /// `#[automatically_derived]`.
    pub(crate) block: Option<&'a PartialBlock>,
    /// The partial impl block a message points at, spelled out.
    pub(crate) block_header: &'a str,
    /// Where an associated item that is not overridden comes from: the
    /// first variant that is always compiled in and whose target has a
    /// type to name, when there is one.
    pub(crate) supplier: Option<Target<'a>>,
    /// The span the impl's own tokens carry: the subject's.
    pub(crate) site: Site,
}

impl Impl<'_> {
    /// Returns `<First as Trait>::name`, spelled at the first variant's
    /// payload so that an error about the projection lands there, or the
    /// error for an enum whose every variant is under `#[cfg]`.
    pub(crate) fn supplied(&self, name: &Ident) -> Result<TokenStream2> {
        let subject = self.subject.ident();
        let Some(first) = self.supplier else {
            return Err(Error::new_spanned(
                subject,
                match self.subject.words() {
                    Words::Enum => format!("every variant of `{subject}` is under `#[cfg]` or reached through a field or a method, so none can supply `{name}` in every build; fix it on the enum with `{name} = ..`"),
                    // A struct has the one field, whose type a route
                    // through a field or a method leaves unnamed.
                    Words::Struct => format!("the field of `{subject}` is reached through a field or a method, so its type cannot be named to supply `{name}`; fix it on the struct with `{name} = ..`"),
                },
            ));
        };
        let payload = first
            .target_type()
            .expect("the supplier is chosen among the targets with a type");
        let trait_path = self.trait_path;
        let span = at(first.payload.span());
        Ok(quote_spanned!(span => <#payload as #trait_path>::#name))
    }
}

/// Returns `generics` with `Param: Trait` added for every target whose
/// payload is the type parameter `Param` of `declared`, each bound once.
fn with_payload_bounds(
    generics: &Generics,
    variants: &[Target],
    declared: &Generics,
    trait_path: &Path,
) -> Generics {
    let mut generics = generics.clone();
    let mut bounded: Vec<&Ident> = vec![];
    for target in variants {
        let Some(parameter) = target.bounded_parameter(declared) else {
            continue;
        };
        if bounded.contains(&parameter) {
            continue;
        }
        bounded.push(parameter);
        let span = at(parameter.span());
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote_spanned!(span => #parameter: #trait_path));
    }
    generics
}
