//! `#[dispatch]` on a subject or on an impl block: what is emitted at the
//! attribute, before any carrier macro runs.
use std::hash::{Hash, Hasher};

use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_crate::FoundCrate;
use quote::{format_ident, quote, ToTokens};
use syn::{Error, Ident, ItemImpl, Meta, PathArguments, Result, Type};

use crate::conversions::conversion_impls;
use crate::options::{is_dispatch_attribute, TraitSpec};
use crate::protocol::{carrier_macro, BlockRequest, CarriedSubject, Fnv, TraitRequest};
use crate::subject::{shape_errors, Subject};

/// The name the facade crate is published under; a renamed dependency is
/// resolved through `proc-macro-crate`.
pub(crate) const FACADE: &str = "whichever";

/// Expands `#[dispatch]` on an enum or a struct. A rejected item is still
/// emitted, with its macro, so that its derives, its uses and its impl
/// blocks resolve and the one real error is not buried under theirs.
pub(crate) fn expand_subject(
    attr: TokenStream2,
    subject: Subject,
    line: usize,
    column: usize,
) -> TokenStream2 {
    let captured = subject.clone().without_item_attributes();
    let emitted = captured.clone().without_inner_attributes();
    let subject_macro = subject_macro(&captured, true, line, column);
    expand_dispatch(attr, subject, line, column).unwrap_or_else(|error| {
        let error = error.to_compile_error();
        quote! { #emitted #subject_macro #error }
    })
}

/// Writes the subject's own macro, which carries its definition, variant
/// and field options included, to `#[dispatch]` on an impl block. Impl
/// blocks live in the subject's crate, so the macro stays crate-local and
/// is reached through the alias. `rejected` tells the impl side that the
/// item itself was rejected, so it must not report the same error again.
pub(crate) fn subject_macro(
    captured: &Subject,
    rejected: bool,
    line: usize,
    column: usize,
) -> TokenStream2 {
    let ident = captured.ident();
    let mut hasher = Fnv::default();
    captured.to_token_stream().to_string().hash(&mut hasher);
    let macro_name = format_ident!(
        "__whichever_dispatch_{}_{:016x}_{}_{}",
        ident,
        hasher.finish(),
        line,
        column
    );
    let carrier = carrier_macro(
        &macro_name,
        quote!(#[allow(unused_macros)]),
        "implement_block",
        CarriedSubject {
            rejected,
            subject: captured,
        },
    );
    quote! {
        #carrier

        #[doc(hidden)]
        #[allow(unused_imports)]
        pub(crate) use #macro_name as #ident;
    }
}

pub(crate) fn expand_dispatch(
    first: TokenStream2,
    mut subject: Subject,
    line: usize,
    column: usize,
) -> Result<TokenStream2> {
    let mut specs = Vec::new();
    if !first.is_empty() {
        specs.push(first);
    }
    let mut kept = vec![];
    for attribute in std::mem::take(subject.attrs_mut()) {
        if !is_dispatch_attribute(&attribute) {
            kept.push(attribute);
            continue;
        }
        match &attribute.meta {
            Meta::List(list) => specs.push(list.tokens.clone()),
            // A bare `#[dispatch]` registers the item without a trait.
            Meta::Path(_) => {}
            Meta::NameValue(_) => {
                return Err(Error::new_spanned(
                    attribute,
                    "expected `#[dispatch(Trait, ...)]`",
                ));
            }
        }
    }
    *subject.attrs_mut() = kept;
    let entries = subject.entries()?;
    let captured = subject;
    let emitted = captured.clone().without_inner_attributes();
    let facade = facade_ident()?;
    let parsed_specs = specs
        .iter()
        .map(|tokens| syn::parse2::<TraitSpec>(tokens.clone()))
        .collect::<Result<Vec<_>>>()?;
    for spec in &parsed_specs {
        spec.check_path()?;
    }
    let generate_from = !parsed_specs.iter().any(|spec| spec.from == Some(false));
    let generate_try_from = parsed_specs.iter().any(|spec| spec.try_from == Some(true));
    let conversions = if captured.has_conversions() {
        conversion_impls(&captured, &entries, generate_from, generate_try_from)?
    } else {
        if parsed_specs.iter().any(TraitSpec::has_switches) {
            return Err(Error::new(
                Span::call_site(),
                "`from` and `try_from` are options of an enum; a struct gets no conversions",
            ));
        }
        TokenStream2::new()
    };
    // Every variant must be forwardable when the item dispatches a trait.
    // The error carries the variant's `cfg`, so a variant that is compiled
    // out is no error, and it is reported here once rather than per trait.
    let shape_errors = if parsed_specs.iter().any(|spec| spec.path.is_some()) {
        shape_errors(&entries)
    } else {
        TokenStream2::new()
    };
    let mut invocations = TokenStream2::new();
    for spec in &parsed_specs {
        let Some(path) = &spec.path else { continue };
        let mut macro_path = path.clone();
        if let Some(last) = macro_path.segments.last_mut() {
            last.arguments = PathArguments::None;
        }
        // The switches stay behind: they were read above and the
        // implementation macro has no use for them.
        let options = spec.to_token_stream();
        let request = TraitRequest {
            facade: &facade,
            subject: &captured,
            spec: &options,
            partial: None,
        };
        invocations.extend(quote! { #macro_path! { #request } });
    }
    let subject_macro = subject_macro(&captured, false, line, column);
    Ok(quote! {
        #emitted
        #conversions
        #subject_macro
        #shape_errors
        #invocations
    })
}

/// Turns `#[dispatch]` on an impl block into a call of the subject's macro,
/// which supplies the subject's definition. An inherent block is forwarded
/// method by method; a trait impl block is a partial impl, whose options
/// travel along for the trait's carrier macro.
pub(crate) fn expand_impl_invocation(
    attr: TokenStream2,
    item_impl: ItemImpl,
) -> Result<TokenStream2> {
    let Type::Path(self_type) = &*item_impl.self_ty else {
        return Err(Error::new_spanned(
            &item_impl.self_ty,
            "the impl must name a dispatch enum or struct by path",
        ));
    };
    let mut macro_path = self_type.path.clone();
    if let Some(last) = macro_path.segments.last_mut() {
        last.arguments = PathArguments::None;
    }
    let facade = facade_ident()?;
    match &item_impl.trait_ {
        Some((Some(bang), _, _)) => Err(Error::new_spanned(
            bang,
            "a negative impl has nothing to dispatch",
        )),
        Some((None, _, _)) => {
            let request = BlockRequest {
                facade: &facade,
                partial: Some(&attr),
                block: &item_impl,
            };
            Ok(quote! { #macro_path! { #request } })
        }
        None => {
            if !attr.is_empty() {
                return Err(Error::new_spanned(
                    attr,
                    "`#[dispatch]` on an inherent impl block takes no arguments; the variants are named on each method",
                ));
            }
            let request = BlockRequest {
                facade: &facade,
                partial: None,
                block: &item_impl,
            };
            Ok(quote! { #macro_path! { #request } })
        }
    }
}

pub(crate) fn facade_ident() -> Result<Ident> {
    match proc_macro_crate::crate_name(FACADE) {
        Ok(FoundCrate::Itself) => Ok(format_ident!("{FACADE}")),
        Ok(FoundCrate::Name(name)) => Ok(Ident::new(&name, Span::call_site())),
        Err(error) => Err(Error::new(
            Span::call_site(),
            format!("`{FACADE}` must be a dependency of this crate: {error}"),
        )),
    }
}
