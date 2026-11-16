//! The items of an impl block under `#[dispatch]`, as read: the methods
//! marked `#[dispatch(..)]`, with the mark taken off, and the rest.
//!
//! An inherent block and a partial impl block hold the same kinds of items
//! and differ in what each does with them, so the reading is one and the
//! two paths decide.
use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Block, Error, ImplItem, ImplItemFn, Result, Signature, Token, Visibility};

use crate::options::is_dispatch_attribute;

/// One item of an impl block under `#[dispatch]`.
// Short-lived and few; the size difference does not matter here.
#[allow(clippy::large_enum_variant)]
pub(crate) enum BlockItem {
    /// A method carrying `#[dispatch(..)]`, with or without a body.
    Marked(Marked),
    /// A method without a body and without a mark, which nothing gives a
    /// body.
    Bodyless(Signature),
    /// Tokens `syn` left verbatim that are no method.
    Unread(TokenStream2),
    /// Any other item, the user's, as written.
    Plain(ImplItem),
}

impl BlockItem {
    /// Reads one item of the block.
    pub(crate) fn read(item: ImplItem) -> Self {
        match item {
            ImplItem::Fn(mut function) => {
                let Some(position) = function.attrs.iter().position(is_dispatch_attribute) else {
                    return BlockItem::Plain(ImplItem::Fn(function));
                };
                let attribute = function.attrs.remove(position);
                BlockItem::Marked(Marked {
                    attribute,
                    attrs: function.attrs.clone(),
                    vis: function.vis.clone(),
                    sig: function.sig.clone(),
                    body: Some(function.block.clone()),
                    written: Some(function),
                })
            }
            // A method without a body is verbatim to `syn`.
            ImplItem::Verbatim(tokens) => {
                let Ok(method) = syn::parse2::<BodylessMethod>(tokens.clone()) else {
                    return BlockItem::Unread(tokens);
                };
                let BodylessMethod {
                    mut attrs,
                    vis,
                    sig,
                } = method;
                let Some(position) = attrs.iter().position(is_dispatch_attribute) else {
                    return BlockItem::Bodyless(sig);
                };
                let attribute = attrs.remove(position);
                BlockItem::Marked(Marked {
                    attribute,
                    attrs,
                    vis,
                    sig,
                    body: None,
                    written: None,
                })
            }
            other => BlockItem::Plain(other),
        }
    }
}

/// A method of an impl block marked `#[dispatch(..)]`.
pub(crate) struct Marked {
    /// The mark, taken off the method.
    pub(crate) attribute: Attribute,
    /// The method's other attributes.
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) vis: Visibility,
    pub(crate) sig: Signature,
    /// The body, which answers for the variants the mark does not name; a
    /// method declared with `;` has none.
    pub(crate) body: Option<Block>,
    /// The method as written, when it has a body.
    written: Option<ImplItemFn>,
}

impl Marked {
    /// Reads the method's one mark with `read`. A second `#[dispatch(..)]`
    /// is reported on itself and taken off too, so that rustc does not
    /// report it as an unknown attribute on top.
    pub(crate) fn mark<T>(&mut self, read: impl FnOnce(&Attribute) -> Result<T>) -> Result<T> {
        let Some(position) = self.attrs.iter().position(is_dispatch_attribute) else {
            return read(&self.attribute);
        };
        let second = self.attrs.remove(position);
        self.attrs
            .retain(|attribute| !is_dispatch_attribute(attribute));
        Err(Error::new_spanned(
            second,
            "one `#[dispatch(..)]` per method; name every variant it forwards to in that one",
        ))
    }

    /// Returns the method as written, without its mark: what stands in
    /// the block when the method cannot be forwarded. A method without a
    /// body leaves nothing, since only a forwarding could give it one.
    pub(crate) fn unmarked(self) -> Option<ImplItem> {
        let mut function = self.written?;
        function.attrs = self.attrs;
        Some(ImplItem::Fn(function))
    }
}

/// A method declared without a body, which `syn` leaves as verbatim tokens.
struct BodylessMethod {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
}

impl Parse for BodylessMethod {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let sig = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self { attrs, vis, sig })
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;
    use crate::options::TraitMark;

    /// The method a marked item reads as, or a panic for any other item.
    fn marked(item: ImplItem) -> Marked {
        match BlockItem::read(item) {
            BlockItem::Marked(method) => method,
            _ => panic!("a marked method"),
        }
    }

    #[test]
    fn a_marked_method_is_told_from_a_bodyless_one_unread_tokens_and_a_plain_item() {
        let with_body = marked(parse_quote! {
            #[inline]
            #[dispatch(*)]
            fn area(&self) -> u32 { 0 }
        });
        assert!(with_body.body.is_some());
        assert_eq!(with_body.attrs.len(), 1, "the mark is taken off");
        assert!(with_body.attribute.path().is_ident("dispatch"));
        let without_body = marked(parse_quote! {
            #[dispatch(Square)]
            fn area(&self) -> u32;
        });
        assert!(without_body.body.is_none() && without_body.attrs.is_empty());
        assert!(matches!(
            BlockItem::read(parse_quote!(fn area(&self) -> u32;)),
            BlockItem::Bodyless(sig) if sig.ident == "area"
        ));
        assert!(matches!(
            BlockItem::read(ImplItem::Verbatim(quote!(
                struct Nested;
            ))),
            BlockItem::Unread(_)
        ));
        assert!(matches!(
            BlockItem::read(parse_quote!(
                fn area(&self) -> u32 {
                    1
                }
            )),
            BlockItem::Plain(ImplItem::Fn(_))
        ));
        assert!(matches!(
            BlockItem::read(parse_quote!(
                const SIDES: u32 = 4;
            )),
            BlockItem::Plain(ImplItem::Const(_))
        ));
    }

    #[test]
    fn a_second_mark_is_reported_and_taken_off_with_every_other() {
        let mut method = marked(parse_quote! {
            #[dispatch(default)]
            #[inline]
            #[dispatch(*)]
            #[dispatch(Square)]
            fn sides(&self) -> u32;
        });
        let error = match method.mark(TraitMark::read) {
            Ok(_) => panic!("a second mark is an error"),
            Err(error) => error.to_string(),
        };
        assert_eq!(
            error,
            "one `#[dispatch(..)]` per method; name every variant it forwards to in that one"
        );
        assert_eq!(method.attrs.len(), 1, "only `#[inline]` is left");
        assert!(method.attrs[0].path().is_ident("inline"));
        let mut once = marked(parse_quote! {
            #[dispatch(default)]
            fn sides(&self) -> u32;
        });
        assert!(matches!(once.mark(TraitMark::read), Ok(TraitMark::Default)));
    }

    #[test]
    fn a_method_stands_unmarked_with_its_body_and_not_without_one() {
        let with_body = marked(parse_quote! {
            #[inline]
            #[dispatch(*)]
            fn area(&self) -> u32 { 0 }
        });
        let Some(ImplItem::Fn(function)) = with_body.unmarked() else {
            panic!("the method as written");
        };
        assert_eq!(function.attrs.len(), 1);
        assert!(function.attrs[0].path().is_ident("inline"));
        assert_eq!(function.sig.ident, "area");
        let without_body = marked(parse_quote! {
            #[dispatch(*)]
            fn area(&self) -> u32;
        });
        assert!(without_body.unmarked().is_none());
    }
}
