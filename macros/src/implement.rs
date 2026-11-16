//! `implement!`: the forwarding impl of one trait for one subject.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{
    parse_quote_spanned, Attribute, Error, Expr, Generics, Ident, Path, Result, Signature,
    TraitItem, TraitItemConst, TraitItemFn, TraitItemType, Type,
};

use crate::agreement::Agreement;
use crate::builds::Builds;
use crate::combined;
use crate::forwarding::{
    ArgumentNames, Arm, ArmContext, Callee, Forwarding, MethodPlan, SelfArguments,
};
use crate::options::{Override, TraitOptions};
use crate::partial::{BlockMethod, Forwarded, PartialBlock};
use crate::protocol::ImplementInput;
use crate::selection::{check_body, select_targets};
use crate::shape::ReturnSite;
use crate::signatures::{carried_attributes, Method, Signatures};
use crate::site::{at, Site};
use crate::spelling;
use crate::subject::{allow_deprecated, is_deprecated, Entries, Subject, Target};
use crate::substitution::Substitution;

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
    let block = partial.map(PartialBlock::read).transpose()?;
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
    let supplier = Supplier {
        first,
        subject: subject.ident(),
        trait_path,
    };
    // Every item of the trait is planned and written in turn; every item
    // that cannot be forwarded is reported, not only the first.
    let mut writer = ItemWriter {
        context: MethodContext {
            entries: &entries,
            subject: &subject,
            subject_path: &subject_path,
            trait_path,
            options: &options,
            block_header: &block_header,
            derived: block.is_none(),
        },
        supplier: &supplier,
        block: block.as_ref(),
        trait_ident,
        signatures: &signatures,
        substitution: Substitution::of_trait(item_trait, trait_path, generics)?,
        site,
        items: TokenStream2::new(),
        agreed_types: vec![],
        agreed_consts: vec![],
        deprecated_items: false,
        plain_forwarding: Builds::Never,
        errors: vec![],
    };
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
    reported.extend(options.unknown_names(
        item_trait,
        trait_ident,
        subject.has_one_target(),
        block.is_none(),
    ));
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

/// One thing an item of the trait becomes in the impl, decided before
/// anything is written. An item the partial block writes in every build,
/// or leaves to the trait's default, has no plan; a method the block
/// writes or marks under `#[cfg]` has one per way it is written.
enum ItemPlan<'a> {
    /// A method marked `#[dispatch(default)]` on a trait that gives it no
    /// default: written with the sentence as its body, under the `#[cfg]`
    /// of the builds the mark is for.
    Rejected {
        method: Method<'a>,
        error: Error,
        builds: Option<TokenStream2>,
    },
    /// An associated type, from the override or the first variant.
    Type(&'a TraitItemType),
    /// An associated const, from the override or the first variant.
    Const(&'a TraitItemConst),
    /// A method, forwarded to every variant or as the block's attribute
    /// names, under the complement of the block's `cfg` when the block
    /// writes or marks it in some builds only. A marked method under its
    /// own `#[cfg]` carries that attribute itself.
    Method {
        method: Method<'a>,
        forwarded: Option<&'a Forwarded>,
        complement: Option<Leftover>,
    },
    /// A shape the impl cannot write.
    Limit(Error),
}

/// The builds the block's conditional items of one name leave to the rest,
/// and the span the attribute naming them carries, the first such item's.
#[derive(Clone)]
struct Leftover {
    builds: Builds,
    span: Span,
}

impl Leftover {
    /// Returns the `#[cfg]` of those builds.
    fn attribute(&self) -> Option<TokenStream2> {
        self.builds.attribute(self.span)
    }
}

/// Writes the items of one trait's impl: plans each item of the trait, then
/// renders the plan, keeping what the agreement checks and the shape errors
/// need to know afterwards.
struct ItemWriter<'a> {
    context: MethodContext<'a>,
    supplier: &'a Supplier<'a>,
    /// The partial block the impl is written around, when there is one.
    block: Option<&'a PartialBlock>,
    trait_ident: &'a Ident,
    /// The trait as carried, which says what it gives each method.
    signatures: &'a Signatures,
    substitution: Substitution,
    /// The span the items' own tokens carry.
    site: Site,
    items: TokenStream2,
    /// The associated types taken from the first variant, for the check.
    agreed_types: Vec<&'a Ident>,
    /// The associated consts taken from the first variant, with their
    /// types, for the check.
    agreed_consts: Vec<(&'a Ident, Type)>,
    /// Whether the trait deprecates an associated item, which the checks
    /// name and so allow.
    deprecated_items: bool,
    /// The builds in which some method with a receiver is forwarded to
    /// every variant. A variant no method can forward to is reported in
    /// those builds and in no other: a forwarding that exists in no build,
    /// the complement of a mark under the trait method's own `cfg`, reports
    /// nothing, and neither does a block whose every method names its
    /// variants.
    plain_forwarding: Builds,
    /// The items that could not be written, which withhold the impl.
    errors: Vec<Error>,
}

impl<'a> ItemWriter<'a> {
    /// Plans and writes one item of the trait.
    fn write(&mut self, item: &'a TraitItem) {
        match item {
            TraitItem::Type(assoc) => self.deprecated_items |= is_deprecated(&assoc.attrs),
            TraitItem::Const(constant) => self.deprecated_items |= is_deprecated(&constant.attrs),
            _ => {}
        }
        for plan in self.plans(item) {
            let site = self.site;
            let generated = match plan {
                ItemPlan::Rejected {
                    method,
                    error,
                    builds,
                } => rejected_method(method, &mut self.substitution, error)
                    .map(|rejected| quote_spanned! { *site => #builds #rejected }),
                ItemPlan::Type(assoc) => associated_type(
                    assoc,
                    self.context.options,
                    self.supplier,
                    self.context.block_header,
                    &mut self.agreed_types,
                    self.site,
                ),
                ItemPlan::Const(constant) => associated_const(
                    constant,
                    self.context.options,
                    self.supplier,
                    &mut self.substitution,
                    &mut self.agreed_consts,
                    self.site,
                ),
                ItemPlan::Method {
                    method,
                    forwarded,
                    complement,
                } => {
                    if forwarded.is_none() && method.function.sig.receiver().is_some() {
                        let builds = match &complement {
                            Some(leftover) => leftover.builds.clone(),
                            None => Builds::Every,
                        };
                        let forwarded =
                            std::mem::replace(&mut self.plain_forwarding, Builds::Never);
                        self.plain_forwarding = forwarded.or(builds);
                    }
                    let complement = complement.as_ref().and_then(Leftover::attribute);
                    expand_method(method, &self.context, &mut self.substitution, forwarded)
                        .map(|forwarded| quote_spanned! { *site => #complement #forwarded })
                }
                ItemPlan::Limit(error) => Err(error),
            };
            match generated {
                Ok(tokens) => self.items.extend(tokens),
                Err(error) => self.errors.push(error),
            }
        }
    }

    /// Decides what `item` becomes. An item the partial block writes is the
    /// block's: in every build when it is written unconditionally, and
    /// where its `cfg` holds when it is written under one, in which case
    /// the forwarded item takes the builds where it does not, as a hand
    /// writes the two arms. A method the block marks is forwarded to the
    /// variants it names around its body, or left to the trait's default,
    /// and a mark under `#[cfg]` is the block's in the same way: it holds
    /// where the `cfg` does, and the builds it leaves go to a mark without
    /// a `#[cfg]`, or to the plain forwarding, so that a payload's override
    /// of a provided method is never lost to the trait's default.
    fn plans(&self, item: &'a TraitItem) -> Vec<ItemPlan<'a>> {
        let name = match item {
            TraitItem::Type(assoc) => &assoc.ident,
            TraitItem::Const(constant) => &constant.ident,
            TraitItem::Fn(function) => &function.sig.ident,
            TraitItem::Macro(item) => {
                return vec![ItemPlan::Limit(Error::new_spanned(
                    item,
                    "macro invocations inside the trait are not supported",
                ))]
            }
            _ => return vec![],
        };
        let name = name.to_string();
        // The builds in which the block writes the item itself.
        let (mut written, mut conditional_at) =
            match self.block.and_then(|block| block.written(&name)) {
                Some(written) if written.builds.is_every() => return vec![],
                Some(written) => (written.builds.clone(), Some(written.span)),
                None => (Builds::Never, None),
            };
        let function = match (item, conditional_at) {
            (TraitItem::Type(assoc), Some(span)) => return vec![ItemPlan::Limit(Error::new(
                span,
                format!("`{}` is declared under `#[cfg]`, and an associated type is declared in every build, since the forwarded methods convert into it; put the whole block under the `cfg`, with a second block for the other builds", assoc.ident),
            ))],
            (TraitItem::Const(constant), Some(span)) => return vec![ItemPlan::Limit(Error::new(
                span,
                format!("`{}` is given under `#[cfg]`, and an associated const is given in every build; put the whole block under the `cfg`, with a second block for the other builds", constant.ident),
            ))],
            (TraitItem::Type(assoc), None) => return vec![ItemPlan::Type(assoc)],
            (TraitItem::Const(constant), None) => return vec![ItemPlan::Const(constant)],
            (TraitItem::Fn(function), _) => self.signatures.method(function),
            _ => return vec![],
        };
        let marked = self
            .block
            .map(|block| block.marked(&name))
            .unwrap_or_default();
        for method in marked {
            let builds = Builds::of(&method.cfgs());
            if !builds.is_every() {
                conditional_at.get_or_insert(method.span());
                written = written.or(builds);
            }
        }
        // The builds the block's conditional items leave.
        let rest = conditional_at.map(|span| Leftover {
            builds: written.complement(),
            span: at(span),
        });
        let site = self.site;
        let mut plans = vec![];
        // Whether a mark without a `#[cfg]` takes those builds.
        let mut taken = false;
        for method in marked {
            let cfgs = method.cfgs();
            taken |= cfgs.is_empty();
            match method {
                BlockMethod::Default { span, .. } => {
                    if function.may_be_left_to_default() {
                        continue;
                    }
                    // Written with the sentence as its body, as a method
                    // that cannot be forwarded is, so that the impl stays
                    // complete.
                    plans.push(ItemPlan::Rejected {
                        method: function,
                        error: Error::new(
                            *span,
                            format!("`{}` has no default body in `{}` to be left to; give it a body, or forward it", function.ident(), self.trait_ident),
                        ),
                        builds: if cfgs.is_empty() {
                            rest.as_ref().and_then(Leftover::attribute)
                        } else {
                            Some(quote_spanned! { *site => #(#cfgs)* })
                        },
                    });
                }
                BlockMethod::Forwarded(forwarded) => plans.push(ItemPlan::Method {
                    method: function,
                    forwarded: Some(forwarded),
                    complement: if cfgs.is_empty() { rest.clone() } else { None },
                }),
            }
        }
        if !taken {
            plans.push(ItemPlan::Method {
                method: function,
                forwarded: None,
                complement: rest,
            });
        }
        plans
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

/// Where an associated item that is not overridden comes from: the first
/// variant that is always compiled in, when there is one.
struct Supplier<'a> {
    first: Option<Target<'a>>,
    subject: &'a Ident,
    trait_path: &'a Path,
}

impl Supplier<'_> {
    /// Returns `<First as Trait>::name`, spelled at the first variant's
    /// payload so that an error about the projection lands there, or the
    /// error for an enum whose every variant is under `#[cfg]`.
    fn item(&self, name: &Ident) -> Result<TokenStream2> {
        let Some(first) = self.first else {
            return Err(Error::new_spanned(
                self.subject,
                format!("every variant of `{}` is under `#[cfg]` or reached through a field or a method, so none can supply `{name}` in every build; fix it on the enum with `{name} = ..`", self.subject),
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

/// Writes `type Name = ..;`: the override, or the supplier's type with
/// `Name` recorded for the agreement check. A generic associated type has
/// no supplier: the enum declares it in a partial impl block, which is
/// where one the block writes was taken out of this path. The item's own
/// tokens carry `site`.
fn associated_type<'a>(
    assoc: &'a TraitItemType,
    options: &TraitOptions,
    supplier: &Supplier,
    block_header: &str,
    agreed: &mut Vec<&'a Ident>,
    site: Site,
) -> Result<TokenStream2> {
    let name = &assoc.ident;
    let mut carried = carried_attributes(&assoc.attrs);
    if is_deprecated(&assoc.attrs) {
        carried.push(allow_deprecated(site));
    }
    // A generic associated type has no supplier: written holding its
    // sentence, as an item no variant can supply is, the impl stays
    // complete and the sentence stands alone. Its parameters and `where`
    // clause are the trait's, which the impl may not strengthen.
    if !assoc.generics.params.is_empty() {
        let parameters = spelling::spelled(&assoc.generics);
        let error = Error::new_spanned(
            assoc,
            format!("`{name}` is a generic associated type, which only the enum can declare; write `type {name}{parameters} = ..;` in a `#[whichever::dispatch] {block_header}` block"),
        )
        .to_compile_error();
        let generics = &assoc.generics;
        let where_clause = &assoc.generics.where_clause;
        return Ok(quote_spanned! { *site =>
            #(#carried)* type #name #generics = [(); { #error; 0 }] #where_clause;
        });
    }
    let cfgs = carried
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"));
    let ty = match options.override_for(name) {
        Some(fixed) => fixed.as_type()?.into_token_stream(),
        None => {
            // The agreement check cannot name an item that may be
            // compiled out.
            if cfgs.count() == 0 {
                agreed.push(name);
            }
            match supplier.item(name) {
                Ok(item) => item,
                // An array whose length is an error is a type rustc takes
                // as erroneous: nothing that names the item is checked
                // further, so the impl is complete, the calls around it
                // resolve, and the sentence stands alone.
                Err(error) => {
                    let error = error.to_compile_error();
                    quote_spanned! { *site => [(); { #error; 0 }] }
                }
            }
        }
    };
    Ok(quote_spanned! { *site => #(#carried)* type #name = #ty; })
}

/// Writes `const NAME: T = ..;`: the override, or the supplier's value with
/// `NAME` recorded for the agreement check. The item's own tokens carry
/// `site`.
fn associated_const<'a>(
    constant: &'a TraitItemConst,
    options: &TraitOptions,
    supplier: &Supplier,
    substitution: &mut Substitution,
    agreed: &mut Vec<(&'a Ident, Type)>,
    site: Site,
) -> Result<TokenStream2> {
    let name = &constant.ident;
    let mut carried = carried_attributes(&constant.attrs);
    if is_deprecated(&constant.attrs) {
        carried.push(allow_deprecated(site));
    }
    let cfgs = carried
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"));
    let mut ty = constant.ty.clone();
    substitution.visit_type_mut(&mut ty);
    substitution.check()?;
    let value = match options.override_for(name) {
        Some(fixed) => fixed.as_value()?.into_token_stream(),
        None => {
            if cfgs.count() == 0 {
                agreed.push((name, ty.clone()));
            }
            // A value that is an error is of any type, as a method's body
            // that is one is.
            match supplier.item(name) {
                Ok(item) => item,
                Err(error) => {
                    let error = error.to_compile_error();
                    quote_spanned! { *site => { #error } }
                }
            }
        }
    };
    Ok(quote_spanned! { *site => #(#carried)* const #name: #ty = #value; })
}

/// Writes the forwarding method for one trait method: a `match` over every
/// variant with a fully qualified call per arm, and the mismatch expression
/// for two different variants when the method takes `Self` arguments. A
/// method without a receiver is left to its default body. A method that
/// cannot be forwarded is still written, with its sentence as its body, so
/// that the impl stays complete, the calls around it resolve, and the one
/// error stands alone; only a signature that cannot be written at all, a
/// projection the substitution could not qualify, is an error of the impl.
fn expand_method(
    method: Method,
    context: &MethodContext,
    substitution: &mut Substitution,
    forwarded: Option<&Forwarded>,
) -> Result<TokenStream2> {
    // A block method's signature is the user's, written where the enum is,
    // and needs no substitution.
    let signature = match forwarded {
        Some(forwarded) => forwarded.sig.clone(),
        None => {
            let mut signature = method.function.sig.clone();
            substitution.visit_signature_mut(&mut signature);
            substitution.check()?;
            signature
        }
    };
    let written = forward_trait_method(method, signature.clone(), context, forwarded);
    Ok(written.unwrap_or_else(|error| {
        let carried = match forwarded {
            Some(forwarded) => forwarded.attrs.clone(),
            None => method.carried_attributes(),
        };
        error_body(&signature, carried, error)
    }))
}

/// Returns the sentence for a method that requires `Self: Sized`
/// forwarded to a trait object, which cannot take it; a hand leaves such a
/// method to the trait's default, and the partial block says so with
/// `#[dispatch(default)]`. It is that variant's arm, under its `cfg`, as any
/// other sentence about a variant is.
fn trait_object_refusal(
    plan: &MethodPlan,
    target: &Target,
    function: &TraitItemFn,
    block_header: &str,
) -> Option<Error> {
    if !plan.requires_sized() || !target.is_trait_object() {
        return None;
    }
    let method = &function.sig.ident;
    Some(Error::new_spanned(
        &function.sig,
        format!("`{method}` requires `Self: Sized`, and `{}` holds a trait object, which is not; leave it to the trait's default with `#[dispatch(default)] fn {method}(..);` in a `#[whichever::dispatch] {block_header}` block, or implement the method by hand", target.ident()),
    ))
}

/// The arm for everything the targets' arms leave, and whether the lint
/// about an unreachable pattern is allowed on the `match`.
struct Fallback {
    arm: Option<Arm>,
    allow_unreachable: bool,
}

/// Decides the fallback arm of a trait method. A block method's body is the
/// arm for every variant it does not name, the mismatch of two different
/// variants included, so it needs no `mismatch`; under `*` without a body
/// the trait's rule applies. Two values of a struct always hold the same
/// field, so only an enum needs the arm for two different variants, and an
/// enum with one variant that is always compiled in has no second variant
/// to meet: a hand writes no fallback arm there, and neither does the
/// engine. The arm stays unreachable in some builds when a variant is under
/// `cfg`, which is the one case the lint is allowed for.
fn fallback_for(
    plan: &MethodPlan,
    entries: &Entries,
    subject: &Subject,
    forwarded: Option<&Forwarded>,
    mismatch: Option<Expr>,
    function: &TraitItemFn,
    context: &ArmContext,
) -> Result<Fallback> {
    let always_present = entries
        .iter()
        .filter(|entry| entry.target().is_some() && !entry.is_conditional())
        .count();
    let conditional = entries
        .iter()
        .any(|entry| entry.is_conditional() || entry.unforwardable().is_some());
    if let Some(body) = forwarded.and_then(|forwarded| forwarded.body.as_ref()) {
        return Ok(Fallback {
            arm: Some(Arm::rest(body.to_token_stream(), context.site)),
            allow_unreachable: false,
        });
    }
    let needs_fallback =
        plan.has_same_variant() && subject.can_mismatch() && (always_present >= 2 || conditional);
    if !needs_fallback {
        return Ok(Fallback {
            arm: None,
            allow_unreachable: false,
        });
    }
    let method = &function.sig.ident;
    let Some(expr) = mismatch else {
        return Err(Error::new_spanned(
            &function.sig,
            format!("`{method}` takes `Self` arguments; give the result for two different variants with `#[dispatch(mismatch = <expr>)]` on the trait method or `mismatch({method} = <expr>)` on the enum"),
        ));
    };
    Ok(Fallback {
        arm: Some(plan.fallback(expr.to_token_stream(), context.site)),
        allow_unreachable: forwarded.is_none() && always_present < 2,
    })
}

/// Writes a trait method that cannot be forwarded as the impl writes it:
/// the trait's signature, substituted, with `error`'s sentence as its body.
/// Only a signature the substitution cannot write is an error of the impl.
fn rejected_method(
    method: Method,
    substitution: &mut Substitution,
    error: Error,
) -> Result<TokenStream2> {
    let mut signature = method.function.sig.clone();
    substitution.visit_signature_mut(&mut signature);
    substitution.check()?;
    let carried = method.carried_attributes();
    Ok(error_body(&signature, carried, error))
}

/// Writes `signature` with `error`'s sentence as its body: rustc treats a
/// `compile_error!` as an error expression and type-checks no further, so
/// the impl is complete, the calls around it resolve, and the sentence
/// stands alone.
fn error_body(signature: &Signature, carried: Vec<Attribute>, error: Error) -> TokenStream2 {
    let error = error.to_compile_error();
    let span = at(signature.ident.span());
    quote_spanned! { span => #(#carried)* #signature { #error } }
}

/// What every method of one trait's impl is written against.
struct MethodContext<'a> {
    entries: &'a Entries<'a>,
    subject: &'a Subject,
    /// The subject's real path, for patterns and values: its own name, or
    /// the path a restatement names.
    subject_path: &'a Path,
    trait_path: &'a Path,
    options: &'a TraitOptions,
    /// The partial impl block a message points at, spelled out.
    block_header: &'a str,
    /// Whether the impl is the enum's attribute's, `#[automatically_derived]`,
    /// rather than a partial impl block's, which is the user's.
    derived: bool,
}

/// Writes the forwarding method for a trait method whose signature is
/// written, or the sentence for the shape that stops it.
fn forward_trait_method(
    declared: Method,
    mut signature: Signature,
    context: &MethodContext,
    forwarded: Option<&Forwarded>,
) -> Result<TokenStream2> {
    let MethodContext {
        entries,
        subject,
        subject_path,
        trait_path,
        options,
        block_header,
        derived,
    } = *context;
    let function = declared.function;
    let subject_ident = subject.ident();
    let method = signature.ident.clone();
    // A method the trait gives a body can be left to it; a restatement
    // lists its methods without one, and they count as required.
    let has_default = declared.has_default();
    if signature.receiver().is_none() {
        if has_default {
            return Ok(TokenStream2::new());
        }
        return Err(Error::new_spanned(
            &function.sig,
            format!("`{method}` has no receiver, so there is no variant to dispatch on; give it a default body, or write it in a `#[whichever::dispatch] {block_header}` block"),
        ));
    }
    let context = ArmContext {
        callee: Callee::Qualified {
            trait_path,
            method: &method,
        },
        method: &method,
        subject: subject_ident,
        prefix: subject_path.clone(),
        lift: None,
        has_body: forwarded.is_some_and(|forwarded| forwarded.body.is_some()),
        site: Site::at(subject_ident.span()),
    };
    // The enum's `mismatch` wins over the trait method's, and the names the
    // fallback arm binds resolve where the chosen expression was written.
    let (mismatch, resolution) = match options.mismatch_for(&method) {
        Some(expr) => (Some(expr.clone()), Some(expr.span())),
        None => (declared.mismatch()?, None),
    };
    // A block method's parameter names are the user's; the trait's are
    // kept where they are plain identifiers and generated otherwise.
    let names = match forwarded {
        Some(_) => ArgumentNames::Written,
        None => ArgumentNames::WrittenOrGenerated,
    };
    let plan = MethodPlan::new(
        &mut signature,
        names,
        SelfArguments::Dispatch,
        Some(options),
        &ReturnSite {
            subject: subject_ident,
            one_target: subject.has_one_target(),
            by_hand: context.whole_way_out(),
            default: has_default.then_some((&method, block_header)),
        },
        resolution,
    )?
    .over(entries);
    // A block method selects its variants and gives the body for the rest;
    // the trait's attribute forwards to every variant.
    let selection = match forwarded {
        Some(forwarded) => {
            let selection = select_targets(
                &forwarded.forwarding,
                entries,
                &context.prefix,
                &method,
                subject,
            )?;
            check_body(
                &selection,
                forwarded.body.as_ref(),
                &signature,
                subject.target_noun(),
            )?;
            Some(selection)
        }
        None => None,
    };
    let arm_for =
        |target: &Target| match trait_object_refusal(&plan, target, function, block_header) {
            Some(error) => plan.sentence_arm(target, &context, error),
            None => plan.arm_or_sentence(target, &context),
        };
    let mut arms = vec![];
    match &selection {
        Some(selection) => {
            for target in &selection.targets {
                arms.push(arm_for(target));
            }
        }
        None => {
            for entry in entries.iter() {
                match entry.target() {
                    Some(target) => arms.push(arm_for(&target)),
                    // The variant is reported elsewhere, under its `cfg`;
                    // this arm only keeps the `match` exhaustive while that
                    // error stands, which the fallback arm does when there
                    // is one.
                    None if !plan.has_same_variant() => {
                        let site = context.site;
                        arms.push(Arm::unforwardable(
                            &context.prefix,
                            entry,
                            plan.standing_in(site),
                            site,
                        ));
                    }
                    None => {}
                }
            }
        }
    }
    let Fallback {
        arm: fallback,
        allow_unreachable,
    } = fallback_for(
        &plan, entries, subject, forwarded, mismatch, function, &context,
    )?;
    if let Some(selection) = selection {
        arms.extend(
            selection
                .unforwardable_arms
                .into_iter()
                .map(|arm| plan.beside_arguments(arm)),
        );
    }
    let site = context.site;
    let allow_unreachable =
        allow_unreachable.then(|| quote_spanned! { *site => #[allow(unreachable_patterns)] });
    // A body for the rest uses the arguments, or is the user's to quiet;
    // the impl the enum's attribute writes is automatically derived.
    let has_body = forwarded.is_some_and(|forwarded| forwarded.body.is_some());
    let allow_unused = match derived || has_body {
        true => None,
        false => plan.allow_unused_when_empty(site),
    };
    // The method's own `#[inline]`, the block's or the trait's, or ours.
    let own_attrs: &[Attribute] = match forwarded {
        Some(forwarded) => &forwarded.attrs,
        None => &function.attrs,
    };
    let inline = own_attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("inline"))
        .map(ToTokens::to_token_stream)
        .unwrap_or_else(|| quote_spanned! { *site => #[inline] });
    let mut carried: Vec<Attribute> = match forwarded {
        Some(forwarded) => forwarded
            .attrs
            .iter()
            .filter(|attribute| !attribute.path().is_ident("inline"))
            .cloned()
            .collect(),
        None => declared.carried_attributes(),
    };
    // A deprecated method is forwarded under `#[allow(deprecated)]`, as a
    // hand writes it; the deprecation reaches the callers through the
    // trait's item.
    if declared.is_deprecated() {
        carried.push(allow_deprecated(site));
    }
    let body = Forwarding {
        scrutinee: plan.scrutinee(context.site),
        arms,
        fallback,
        future: plan.future.clone(),
    }
    .body(Site::at(method.span()));
    Ok(quote_spanned! { *site =>
        #(#carried)*
        #inline
        #allow_unreachable
        #allow_unused
        #signature #body
    })
}
