//! `#[dispatchable]`: the carrier macro a trait leaves next to itself.
use proc_macro2::{Group, Punct, Spacing, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{Error, ItemTrait, Result, TraitItem, TraitItemFn, Visibility};

use crate::combined;
use crate::naming;
use crate::options::{is_dispatch_attribute, method_mismatch, DispatchableOptions};
use crate::protocol::{carrier_macro, CarriedTrait, Implementation};
use crate::shape::takes_self_argument;

pub(crate) fn expand_dispatchable(
    item_trait: ItemTrait,
    options: DispatchableOptions,
    line: usize,
    column: usize,
) -> Result<TokenStream2> {
    let captured = item_trait.clone();
    let mut emitted = item_trait;
    let mut errors = vec![];
    for item in &mut emitted.items {
        if let TraitItem::Fn(function) = item {
            if let Err(error) = check_method_attributes(function) {
                errors.push(error);
            }
            function
                .attrs
                .retain(|attribute| !is_dispatch_attribute(attribute));
        }
    }
    // A trait with a bad attribute is still emitted, so that its impls
    // compile; without the carrier, dispatching it reports the missing
    // macro, which this error explains.
    if let Some(error) = combined(errors) {
        let error = error.to_compile_error();
        let emitted = options.remote.is_none().then_some(&emitted);
        return Ok(quote! { #emitted #error });
    }
    let trait_ident = emitted.ident.clone();
    let vis = emitted.vis.clone();
    // A `pub` trait may be dispatched from another crate, which reaches a
    // `macro_rules!` only through `#[macro_export]`; the macro then lives at
    // the crate root, so its name carries the trait's text, the position of
    // every token and the call site to keep same-named traits apart. A trait
    // of any other visibility cannot leave the crate, so its macro stays a
    // plain `macro_rules!` in the trait's module, reachable through the
    // re-export wherever the trait is, and adds nothing to the crate root.
    let export = if matches!(vis, Visibility::Public(_)) {
        quote!(#[macro_export])
    } else {
        quote!(#[allow(unused_macros)])
    };
    let captured = captured.to_token_stream();
    let name = naming::trait_carrier(&trait_ident, &captured, options.name.as_ref(), line, column);
    // The captured tokens expand inside the enum's crate, where `crate::`
    // would name the wrong crate; `$crate` resolves to this crate instead.
    let captured = rewrite_crate_paths(captured);
    // A restated trait exists elsewhere; only its macro is emitted here, and
    // the impl is written for the real path.
    let remote = options
        .remote
        .map(|path| rewrite_crate_paths(path.to_token_stream()));
    let emitted = remote.is_none().then_some(emitted);
    let carrier = carrier_macro(
        &name,
        quote!(#[doc(hidden)] #export),
        Implementation::Trait,
        CarriedTrait {
            tokens: &captured,
            remote: remote.as_ref(),
        },
    );
    Ok(quote! {
        #emitted

        #carrier

        #[doc(hidden)]
        #[allow(unused_imports)]
        #vis use #name as #trait_ident;
    })
}

/// Checks a trait method's `#[dispatch(..)]`: one at most, `mismatch = expr`
/// only, and only on a method with a `Self` argument to decide for.
fn check_method_attributes(function: &TraitItemFn) -> Result<()> {
    let mut attributes = function
        .attrs
        .iter()
        .filter(|attribute| is_dispatch_attribute(attribute));
    let Some(first) = attributes.next() else {
        return Ok(());
    };
    if let Some(second) = attributes.next() {
        return Err(Error::new_spanned(
            second,
            "one `#[dispatch]` attribute per method",
        ));
    }
    if method_mismatch(function)?.is_some() && !takes_self_argument(&function.sig) {
        return Err(Error::new_spanned(
            first,
            format!(
                "`mismatch` decides between two different variants, and `{}` takes no `Self`, `&Self` or `&mut Self` argument",
                function.sig.ident
            ),
        ));
    }
    Ok(())
}

/// Replaces every `crate::` path prefix in `tokens` with `$crate::`.
fn rewrite_crate_paths(tokens: TokenStream2) -> TokenStream2 {
    let mut output = TokenStream2::new();
    let mut iter = tokens.into_iter().peekable();
    while let Some(token) = iter.next() {
        match token {
            TokenTree::Group(group) => {
                let mut rewritten =
                    Group::new(group.delimiter(), rewrite_crate_paths(group.stream()));
                rewritten.set_span(group.span());
                output.extend([TokenTree::Group(rewritten)]);
            }
            TokenTree::Ident(ident)
                if ident == "crate"
                    && matches!(iter.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ':') =>
            {
                output.extend([
                    TokenTree::Punct(Punct::new('$', Spacing::Alone)),
                    TokenTree::Ident(ident),
                ]);
            }
            other => output.extend([other]),
        }
    }
    output
}
