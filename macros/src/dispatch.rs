//! `#[dispatch]` on a subject or on an impl block: what is emitted at the
//! attribute, before any carrier macro runs.
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_crate::FoundCrate;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Error, Ident, ImplItem, ItemImpl, Meta, Path, Result, Type};

use crate::combined;
use crate::conversions::conversion_impls;
use crate::naming;
use crate::options::{is_dispatch_attribute, DispatchAttribute, Switch, TraitSpec};
use crate::protocol::{
    carrier_macro, carrier_path, BlockRequest, CarriedSubject, Implementation, TraitRequest,
};
use crate::subject::{shape_errors, Entry, Subject};

/// The name the facade crate is published under; a renamed dependency is
/// resolved through `proc-macro-crate`.
pub(crate) const FACADE: &str = "whichever";

/// Expands `#[dispatch]` on an enum or a struct. Every error is reported
/// and nothing that can still be written is withheld: the item itself, its
/// carrier, the conversions and the impls of the attributes that parsed are
/// emitted around the errors, so that the derives, the uses, the impl
/// blocks and the calls around the item resolve and each error stands
/// alone. Only a subject whose variants or field cannot be read is
/// rejected, which tells its impl blocks not to report the same error.
pub(crate) fn expand_subject(
    attr: TokenStream2,
    mut subject: Subject,
    line: usize,
    column: usize,
) -> TokenStream2 {
    let (arguments, mut errors) = take_dispatch_attributes(&mut subject, attr);
    // The subject as the carriers hold it, variant and field options
    // included, and as it is emitted, without them.
    let captured = subject;
    let emitted = captured.clone().without_inner_attributes();
    let readable = captured
        .entries()
        .and_then(|entries| Ok((entries, facade_ident()?)));
    let (entries, facade) = match readable {
        Ok(readable) => readable,
        Err(error) => {
            errors.push(error);
            let carrier = subject_carrier(&captured, true, &[], line, column);
            let errors = combined(errors).map(|error| error.to_compile_error());
            return quote! { #emitted #carrier #errors };
        }
    };
    let mut attributes = vec![];
    for tokens in arguments {
        let parsed = syn::parse2::<DispatchAttribute>(tokens).and_then(|attribute| {
            attribute.spec.check_path()?;
            Ok(attribute)
        });
        match parsed {
            Ok(attribute) => attributes.push(attribute),
            Err(error) => errors.push(error),
        }
    }
    let conversions = conversions(&captured, &entries, &attributes, &mut errors);
    let dispatched: Vec<&Path> = attributes
        .iter()
        .filter_map(|attribute| attribute.spec.path.as_ref())
        .collect();
    // Every variant must be forwardable when the item dispatches a trait.
    // The error carries the variant's `cfg`, so a variant that is compiled
    // out is no error, and it is reported here once rather than per trait.
    let shape_errors = if dispatched.is_empty() {
        TokenStream2::new()
    } else {
        shape_errors(&entries)
    };
    let mut invocations = TokenStream2::new();
    for attribute in &attributes {
        let Some(path) = &attribute.spec.path else {
            continue;
        };
        let macro_path = carrier_path(path);
        // The switches stay behind: they were read above and the
        // implementation macro has no use for them.
        let request = TraitRequest {
            facade: &facade,
            subject: &captured,
            spec: &attribute.spec,
            partial: None,
        };
        // The invocation carries the span of its own attribute, so that an
        // error rustc finds in this trait's impl is labelled there and not
        // on the first attribute of the stack.
        let site = path.span();
        invocations.extend(quote_spanned! { site => #macro_path! { #request } });
    }
    let carrier = subject_carrier(&captured, false, &dispatched, line, column);
    let errors = combined(errors).map(|error| error.to_compile_error());
    quote! {
        #emitted
        #conversions
        #carrier
        #shape_errors
        #invocations
        #errors
    }
}

/// Takes every `#[dispatch]` attribute off the item and returns the
/// arguments of each, with the errors of those written in a form the
/// attribute does not take; `first` is the one the proc macro was invoked by.
fn take_dispatch_attributes(
    subject: &mut Subject,
    first: TokenStream2,
) -> (Vec<TokenStream2>, Vec<Error>) {
    let mut arguments = vec![];
    if !first.is_empty() {
        arguments.push(first);
    }
    let mut kept = vec![];
    let mut errors = vec![];
    for attribute in std::mem::take(subject.attrs_mut()) {
        if !is_dispatch_attribute(&attribute) {
            kept.push(attribute);
            continue;
        }
        match &attribute.meta {
            Meta::List(list) => arguments.push(list.tokens.clone()),
            // A bare `#[dispatch]` registers the item without a trait.
            Meta::Path(_) => {}
            Meta::NameValue(_) => {
                errors.push(Error::new_spanned(
                    &attribute,
                    "expected `#[dispatch(Trait, ...)]`",
                ));
            }
        }
    }
    *subject.attrs_mut() = kept;
    (arguments, errors)
}

/// Writes the `From` and `TryFrom` impls the attributes ask for: an enum's,
/// switched by `from` and `try_from` on any of its attributes; a struct has
/// none, and a switch on one is an error.
fn conversions(
    subject: &Subject,
    entries: &[Entry],
    attributes: &[DispatchAttribute],
    errors: &mut Vec<Error>,
) -> TokenStream2 {
    if !subject.has_conversions() {
        if let Some(switch) = attributes
            .iter()
            .find_map(|attribute| attribute.switches.first())
        {
            errors.push(Error::new(
                switch.span(),
                "`from` and `try_from` are options of an enum; a struct gets no conversions",
            ));
        }
        return TokenStream2::new();
    }
    let switch = |pick: fn(&DispatchAttribute) -> Option<&Switch>, value: bool| {
        attributes
            .iter()
            .filter_map(pick)
            .any(|switch| switch.value() == value)
    };
    let from = !switch(|attribute| attribute.switches.from.as_ref(), false);
    let try_from = switch(|attribute| attribute.switches.try_from.as_ref(), true);
    conversion_impls(subject, entries, from, try_from, errors)
}

/// Writes the subject's own carrier macro, which carries its definition,
/// variant and field options included, to `#[dispatch]` on an impl block.
/// Impl blocks live in the subject's crate, so the macro stays crate-local
/// and is reached through the alias. `rejected` tells the block side that
/// the item itself was rejected, so it must not report the same error again;
/// `dispatched` names the traits the item's own attributes implement, so
/// that a partial block of one of them is reported as the second impl.
fn subject_carrier(
    captured: &Subject,
    rejected: bool,
    dispatched: &[&Path],
    line: usize,
    column: usize,
) -> TokenStream2 {
    let ident = captured.ident();
    let name = naming::subject_carrier(ident, &captured.to_token_stream(), line, column);
    let carrier = carrier_macro(
        &name,
        quote!(#[allow(unused_macros)]),
        Implementation::Block,
        CarriedSubject {
            rejected,
            dispatched,
            subject: captured,
        },
    );
    quote! {
        #carrier

        #[doc(hidden)]
        #[allow(unused_imports)]
        pub(crate) use #name as #ident;
    }
}

/// Expands `#[dispatch]` on an impl block. A rejected inherent block is
/// still emitted, without the method attributes that are this crate's, so
/// that the methods it declares resolve and the one real error stands
/// alone. A rejected trait impl block is incomplete without the forwarding,
/// which rustc would report on top, so it is dropped.
pub(crate) fn expand_block(attr: TokenStream2, item_impl: ItemImpl) -> TokenStream2 {
    let kept = item_impl
        .trait_
        .is_none()
        .then(|| without_method_attributes(item_impl.clone()));
    expand_impl_invocation(attr, item_impl).unwrap_or_else(|error| {
        let error = error.to_compile_error();
        quote! { #kept #error }
    })
}

/// Returns the block without `#[dispatch]` on its methods, and without the
/// methods declared without a body, which only a forwarding could give one.
fn without_method_attributes(mut item_impl: ItemImpl) -> ItemImpl {
    item_impl
        .items
        .retain(|item| !matches!(item, ImplItem::Verbatim(_)));
    for item in &mut item_impl.items {
        if let ImplItem::Fn(function) = item {
            function
                .attrs
                .retain(|attribute| !is_dispatch_attribute(attribute));
        }
    }
    item_impl
}

/// Turns `#[dispatch]` on an impl block into a call of the subject's
/// carrier, which supplies the subject's definition. An inherent block is
/// forwarded method by method; a trait impl block is a partial impl, whose
/// options travel along for the trait's carrier macro.
fn expand_impl_invocation(attr: TokenStream2, item_impl: ItemImpl) -> Result<TokenStream2> {
    let Type::Path(self_type) = &*item_impl.self_ty else {
        return Err(Error::new_spanned(
            &item_impl.self_ty,
            "the impl must name a dispatch enum or struct by path",
        ));
    };
    let macro_path = carrier_path(&self_type.path);
    let facade = facade_ident()?;
    match &item_impl.trait_ {
        Some((Some(bang), _, _)) => Err(Error::new_spanned(
            bang,
            "a negative impl has nothing to dispatch",
        )),
        Some((None, _, _)) => {
            // The trait's options; the trait itself is the one the block
            // implements.
            let spec: TraitSpec = syn::parse2(attr)?;
            if let Some(path) = &spec.path {
                return Err(Error::new_spanned(
                    path,
                    "`#[dispatch]` on an impl block takes the trait's options only; the trait is the one the block implements",
                ));
            }
            let request = BlockRequest {
                facade: &facade,
                partial: Some(&spec),
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
