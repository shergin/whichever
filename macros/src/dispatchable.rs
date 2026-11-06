//! `#[dispatchable]`: the carrier macro a trait leaves next to itself.
use proc_macro2::{Group, Punct, Spacing, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{Error, Ident, ItemTrait, Result, TraitItem, TraitItemFn, Visibility};

use crate::combined;
use crate::naming;
use crate::options::{is_dispatch_attribute, method_mismatch, DispatchableOptions};
use crate::protocol::{carrier_macro, CarriedTrait, Implementation};
use crate::shape::takes_self_argument;
use crate::subject::{carried_attributes, cfg_attributes};

/// The most distinct `#[cfg]` predicates a trait's items may carry: each
/// doubles the carriers, since one is emitted per combination.
const MOST_PREDICATES: usize = 4;

pub(crate) fn expand_dispatchable(
    item_trait: ItemTrait,
    options: DispatchableOptions,
    line: usize,
    column: usize,
) -> Result<TokenStream2> {
    let captured = item_trait.clone();
    let mut emitted = item_trait;
    let mut errors = vec![];
    if let Err(error) = check_attribute_order(&emitted) {
        errors.push(error);
    }
    let predicates = match predicates(&captured) {
        Ok(predicates) => predicates,
        Err(error) => {
            errors.push(error);
            vec![]
        }
    };
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
    let captured_tokens = captured.to_token_stream();
    let name = naming::trait_carrier(
        &trait_ident,
        &captured_tokens,
        options.name.as_ref(),
        line,
        column,
        &naming::written_at,
    );
    // A restated trait exists elsewhere; only its macro is emitted here, and
    // the impl is written for the real path.
    let remote = options
        .remote
        .map(|path| rewrite_crate_paths(path.to_token_stream()));
    let emitted = remote.is_none().then_some(emitted);
    // A `#[cfg]` on a trait item is decided here, where its predicate means
    // what the author meant, and not in the enum's crate: the carrier is
    // emitted once per combination of the distinct predicates, each under
    // the `cfg` of its combination and holding the items that exist under
    // it, with their `cfg` attributes taken off. rustc keeps exactly one.
    let carriers = (0..1usize << predicates.len()).map(|combination| {
        let holds = |index: usize| combination & (1 << index) != 0;
        let condition = (!predicates.is_empty()).then(|| {
            let terms = predicates.iter().enumerate().map(|(index, predicate)| {
                let predicate = &predicate.tokens;
                if holds(index) {
                    quote!(#predicate)
                } else {
                    quote!(not(#predicate))
                }
            });
            quote!(#[cfg(all(#(#terms),*))])
        });
        let mut selected = captured.clone();
        selected
            .items
            .retain(|item| match predicate_of(item, &predicates) {
                Some(index) => holds(index),
                None => true,
            });
        let provided = signatures_only(&mut selected);
        // The captured tokens expand inside the enum's crate, where
        // `crate::` would name the wrong crate; `$crate` resolves to this
        // crate instead.
        let selected = rewrite_crate_paths(selected.to_token_stream());
        carrier_macro(
            &name,
            quote!(#condition #[doc(hidden)] #export),
            Implementation::Trait,
            CarriedTrait {
                tokens: &selected,
                remote: remote.as_ref(),
                provided: &provided,
            },
        )
    });
    let carriers: Vec<TokenStream2> = carriers.collect();
    Ok(quote! {
        #emitted

        #(#carriers)*

        #[doc(hidden)]
        #[allow(unused_imports)]
        #vis use #name as #trait_ident;
    })
}

/// Checks the order of `#[dispatchable]` among the attribute macros whose
/// order it depends on. `#[async_trait]` below it would rewrite the
/// signatures after the carrier captured them, so the forwarding impl would
/// not match the trait; it goes above. `#[typetag::serde]` above it has
/// added typetag's per-impl methods to the trait before the carrier
/// captured it, and a forwarder of those would answer with the payload's
/// tag, silently; it goes below, and the block's own `#[typetag::serde(name
/// = ..)]` writes those methods.
fn check_attribute_order(item_trait: &ItemTrait) -> Result<()> {
    if let Some(attribute) = item_trait.attrs.iter().find(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "async_trait")
    }) {
        return Err(Error::new_spanned(
            attribute,
            "`#[async_trait]` rewrites the signatures after `whichever` has read them; put `#[whichever::dispatchable]` below it",
        ));
    }
    let hidden_method = |name: &str| {
        item_trait.items.iter().any(|item| {
            matches!(item, TraitItem::Fn(function) if function.sig.ident == name
                && function.attrs.iter().any(|attribute| attribute.path().is_ident("doc")))
        })
    };
    if hidden_method("typetag_name") && hidden_method("typetag_deserialize") {
        return Err(Error::new_spanned(
            &item_trait.ident,
            "`#[typetag::serde]` has added its per-impl methods to the trait, which a forwarder would answer with the payload's tag; put `#[whichever::dispatchable]` above it, and `#[typetag::serde(name = \"..\")]` on the enum's partial impl block",
        ));
    }
    Ok(())
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

/// Strips the trait to what the implementation macro reads: the header's
/// generics, and each item's signature with the attributes the generated
/// item carries, `#[inline]`, a hand-written `#[allow]` and the method's
/// own `#[dispatch]`. Default bodies, doc comments, supertraits, bounds on
/// associated types and the other attributes stay in the trait, where they
/// belong, and out of the exported macro. Returns the methods that had a
/// default, which the carrier names, since a method without a receiver is
/// left to its default and is otherwise a limit. The `cfg` attributes are
/// decided by the carrier's own `cfg` and are dropped here.
fn signatures_only(item_trait: &mut ItemTrait) -> Vec<Ident> {
    item_trait.attrs.clear();
    item_trait.supertraits.clear();
    let mut provided = vec![];
    for item in &mut item_trait.items {
        match item {
            TraitItem::Fn(function) => {
                if function.default.take().is_some() {
                    provided.push(function.sig.ident.clone());
                    function.semi_token = Some(Default::default());
                }
                function.attrs = kept_attributes(&function.attrs, true);
            }
            TraitItem::Type(assoc) => {
                assoc.default = None;
                assoc.bounds.clear();
                assoc.colon_token = None;
                assoc.attrs = kept_attributes(&assoc.attrs, false);
            }
            TraitItem::Const(constant) => {
                constant.default = None;
                constant.attrs = kept_attributes(&constant.attrs, false);
            }
            _ => {}
        }
    }
    provided
}

/// Returns the attributes of a trait item the implementation macro reads:
/// the hand-written `#[allow]`, the `#[deprecated]` the generated item
/// allows, and on a method its `#[inline]` and its `#[dispatch]`.
fn kept_attributes(attrs: &[syn::Attribute], on_method: bool) -> Vec<syn::Attribute> {
    // The `cfg` attributes were decided by the carrier's own `cfg`.
    let mut kept: Vec<syn::Attribute> = carried_attributes(attrs)
        .into_iter()
        .filter(|attribute| !attribute.path().is_ident("cfg"))
        .collect();
    kept.extend(
        attrs
            .iter()
            .filter(|attribute| {
                attribute.path().is_ident("deprecated")
                    || (on_method
                        && (attribute.path().is_ident("inline")
                            || is_dispatch_attribute(attribute)))
            })
            .cloned(),
    );
    kept
}

/// One distinct `#[cfg]` predicate among a trait's items; several `cfg`
/// attributes on one item are `all` of their predicates.
struct Predicate {
    key: String,
    tokens: TokenStream2,
}

/// Returns the distinct predicates of the trait's items, in order of first
/// appearance, or the error for a trait with more than the carriers can
/// reasonably double for.
fn predicates(item_trait: &ItemTrait) -> Result<Vec<Predicate>> {
    let mut predicates: Vec<Predicate> = vec![];
    for item in &item_trait.items {
        let Some(tokens) = item_predicate(item) else {
            continue;
        };
        let key = tokens.to_string();
        if predicates.iter().any(|predicate| predicate.key == key) {
            continue;
        }
        if predicates.len() == MOST_PREDICATES {
            return Err(Error::new_spanned(
                item,
                format!("this is the {}th distinct `#[cfg]` predicate among the trait's items, and the trait's carrier macro is written once per combination of them; combine the predicates, or move the items under this one to a trait of their own", MOST_PREDICATES + 1),
            ));
        }
        predicates.push(Predicate { key, tokens });
    }
    Ok(predicates)
}

/// Returns the index of the predicate `item` is under, when it is under one.
fn predicate_of(item: &TraitItem, predicates: &[Predicate]) -> Option<usize> {
    let key = item_predicate(item)?.to_string();
    predicates.iter().position(|predicate| predicate.key == key)
}

/// Returns the one predicate `item`'s `cfg` attributes amount to.
fn item_predicate(item: &TraitItem) -> Option<TokenStream2> {
    let attrs = item_attrs(item)?;
    let cfgs = cfg_attributes(attrs);
    if cfgs.is_empty() {
        return None;
    }
    let inner: Vec<TokenStream2> = cfgs
        .iter()
        .map(|attribute| match &attribute.meta {
            syn::Meta::List(list) => list.tokens.clone(),
            _ => TokenStream2::new(),
        })
        .collect();
    Some(match inner.as_slice() {
        [one] => one.clone(),
        many => quote!(all(#(#many),*)),
    })
}

fn item_attrs(item: &TraitItem) -> Option<&Vec<syn::Attribute>> {
    match item {
        TraitItem::Fn(function) => Some(&function.attrs),
        TraitItem::Type(assoc) => Some(&assoc.attrs),
        TraitItem::Const(constant) => Some(&constant.attrs),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn the_carrier_holds_the_signatures_and_nothing_else() {
        let item_trait: ItemTrait = parse_quote!(
            /// The doc comment stays with the trait.
            pub trait Counted: Clone {
                /// So does this one.
                type Tally: Default;
                const START: u32 = 7;
                #[inline(always)]
                #[must_use]
                fn count(&self) -> u32;
                #[dispatch(mismatch = false)]
                fn same(&self, other: &Self) -> bool {
                    let secret = 41;
                    secret + 1 == 42
                }
            }
        );
        let options: DispatchableOptions = syn::parse_str("name = unit").unwrap();
        let expanded = expand_dispatchable(item_trait, options, 1, 1)
            .unwrap()
            .to_string();
        let once = |needle: &str| expanded.matches(needle).count();
        assert_eq!(
            once("secret"),
            2,
            "the body is in the trait, not the carrier"
        );
        assert_eq!(once("doc comment"), 1);
        assert_eq!(once("So does"), 1);
        assert_eq!(once("must_use"), 1);
        assert_eq!(once("Default"), 1, "the bound stays with the trait");
        assert_eq!(once("Clone"), 1, "so does the supertrait");
        assert_eq!(once("inline"), 2, "the method's own `#[inline]` travels");
        assert_eq!(
            once("mismatch"),
            1,
            "the trait's `#[dispatch]` is consumed; the carrier's stays"
        );
        assert!(expanded.contains("provided { same }"));
    }
}
