//! `#[dispatchable]`: the carrier macro a trait leaves next to itself.
use proc_macro2::{Group, Punct, Spacing, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{ItemTrait, Result, TraitItem, Visibility};

use crate::naming;
use crate::options::{is_dispatch_attribute, DispatchableOptions};
use crate::protocol::{carrier_macro, CarriedTrait, Implementation};

pub(crate) fn expand_dispatchable(
    item_trait: ItemTrait,
    options: DispatchableOptions,
    line: usize,
    column: usize,
) -> Result<TokenStream2> {
    let captured = item_trait.clone();
    let mut emitted = item_trait;
    for item in &mut emitted.items {
        if let TraitItem::Fn(function) = item {
            function
                .attrs
                .retain(|attribute| !is_dispatch_attribute(attribute));
        }
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
