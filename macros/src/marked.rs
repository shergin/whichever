//! The items of an impl block under `#[dispatch]`, as read: the methods
//! marked `#[dispatch(..)]`, with the mark taken off, and the rest.
//!
//! An inherent block and a partial impl block hold the same kinds of items
//! and differ in what each does with them, so the reading is one and the
//! two paths decide.
use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::{
    parse_quote_spanned, Attribute, Block, Error, Expr, FnArg, Ident, ImplItem, ImplItemFn, Pat,
    Result, Signature, Stmt, Token, Visibility,
};

use crate::arm::receiver_scrutinee;
use crate::options::{is_dispatch_attribute, ForwardingOptions};
use crate::selection::OwnArms;
use crate::shape::{classify_receiver, ReceiverKind};
use crate::site::Site;

/// One item of an impl block under `#[dispatch]`.
// Short-lived and few; the size difference does not matter here.
#[allow(clippy::large_enum_variant)]
pub(crate) enum BlockItem {
    /// A method carrying `#[dispatch(..)]`, with or without a body.
    Marked(Marked),
    /// A method without a body and without a mark, which nothing gives a
    /// body, with the attributes it is written under.
    Bodyless {
        attrs: Vec<Attribute>,
        sig: Signature,
    },
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
                    return BlockItem::Bodyless { attrs, sig };
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
    /// body leaves nothing, since only a forwarding could give it one, and
    /// one whose body holds arms of its own is closed, see `standing_alone`.
    pub(crate) fn unmarked(self) -> Option<ImplItem> {
        let mut function = self.written?;
        function.attrs = self.attrs;
        standing_alone(&mut function, &self.attribute);
        Some(ImplItem::Fn(function))
    }
}

/// Closes the body of `function`, a method kept as written without its
/// `mark`, where that body holds arms of the method's own. Under `*` a
/// body that is one `match self` names only the variants the enum answers
/// itself and is exhaustive only with the forwarding arms written into
/// it, so as written it would be rustc's E0004 beside the error that kept
/// the method. An arm for the rest stands in for the forwarding, reading
/// the arguments and under the allowance that keeps it quiet where the
/// arms already name every variant, and the `match` looks at what the
/// forwarding's would: the enum moved out of its box under `self:
/// Box<Self>`, or the pinned reference unpinned, which `match self` as
/// written is not.
pub(crate) fn standing_alone(function: &mut ImplItemFn, mark: &Attribute) {
    let under_star = mark
        .parse_args::<ForwardingOptions>()
        .is_ok_and(|forwarding| forwarding.all);
    if !under_star || OwnArms::written(&function.block).is_none() {
        return;
    }
    let site = Site::at(function.sig.ident.span());
    let scrutinee =
        function
            .sig
            .receiver()
            .and_then(|receiver| match classify_receiver(receiver).ok()? {
                ReceiverKind::Value(_) => None,
                kind => Some(receiver_scrutinee(kind, &receiver.self_token, site)),
            });
    // The closing arm reads the arguments, as the arm standing in for a
    // variant's own error does: where the only arm that uses one is
    // compiled out, the lint about it would stand beside the error.
    let arguments: Vec<Ident> = function
        .sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(argument) => match &*argument.pat {
                Pat::Ident(binding) => Some(binding.ident.clone()),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect();
    let rest: Expr = if arguments.is_empty() {
        parse_quote_spanned!(*site => ::core::unreachable!())
    } else {
        parse_quote_spanned!(*site => {
            let _ = (#(&#arguments,)*);
            ::core::unreachable!()
        })
    };
    if let [Stmt::Expr(Expr::Match(written), None)] = function.block.stmts.as_mut_slice() {
        if let Some(scrutinee) = scrutinee {
            *written.expr = Expr::Verbatim(scrutinee);
        }
        written.arms.push(parse_quote_spanned! { *site =>
            #[allow(unreachable_patterns)]
            _ => #rest,
        });
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
    use quote::{quote, ToTokens};
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
            BlockItem::Bodyless { sig, .. } if sig.ident == "area"
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

    /// The method `item` as `standing_alone` leaves it once its mark is
    /// taken off, spelled.
    fn standing(item: ImplItemFn) -> (ImplItemFn, String) {
        let mut function = item;
        let position = function
            .attrs
            .iter()
            .position(is_dispatch_attribute)
            .expect("a mark");
        let mark = function.attrs.remove(position);
        standing_alone(&mut function, &mark);
        let spelled = quote!(#function).to_string();
        (function, spelled)
    }

    #[test]
    fn a_methods_own_arms_under_star_stand_alone_closed_by_an_arm_for_the_rest() {
        let (function, spelled) = standing(parse_quote! {
            #[dispatch(*)]
            fn len(&self) -> usize {
                match self {
                    Self::Empty => 0,
                }
            }
        });
        let [Stmt::Expr(Expr::Match(closed), None)] = function.block.stmts.as_slice() else {
            panic!("one `match self`: {spelled}");
        };
        assert_eq!(closed.arms.len(), 2, "{spelled}");
        let rest = &closed.arms[1];
        let attributes = &rest.attrs;
        assert!(matches!(rest.pat, syn::Pat::Wild(_)), "{spelled}");
        assert_eq!(
            quote!(#(#attributes)*).to_string(),
            quote!(#[allow(unreachable_patterns)]).to_string(),
            "the arms may already name every variant"
        );
        assert_eq!(
            rest.body.to_token_stream().to_string(),
            quote!(::core::unreachable!()).to_string()
        );
    }

    /// The expression the kept `match` of `item` looks at, spelled.
    fn scrutinee(item: ImplItemFn) -> String {
        let (function, spelled) = standing(item);
        let [Stmt::Expr(Expr::Match(closed), None)] = function.block.stmts.as_slice() else {
            panic!("one `match`: {spelled}");
        };
        assert_eq!(
            closed.arms.len(),
            2,
            "closed by an arm for the rest: {spelled}"
        );
        closed.expr.to_token_stream().to_string()
    }

    #[test]
    fn a_kept_match_looks_at_what_the_forwarding_would_under_its_receiver() {
        assert_eq!(
            scrutinee(parse_quote! {
                #[dispatch(*)]
                fn len(&self) -> usize {
                    match self {
                        Self::Empty => 0,
                    }
                }
            }),
            quote!(self).to_string(),
            "a reference is matched as written"
        );
        assert_eq!(
            scrutinee(parse_quote! {
                #[dispatch(*)]
                fn len(self) -> usize {
                    match self {
                        Self::Empty => 0,
                    }
                }
            }),
            quote!(self).to_string(),
            "a value is matched as written"
        );
        assert_eq!(
            scrutinee(parse_quote! {
                #[dispatch(*)]
                fn take(self: Box<Self>) -> usize {
                    match self {
                        Self::Empty => 0,
                    }
                }
            }),
            quote!(*self).to_string(),
            "the enum is moved out of its box"
        );
        assert_eq!(
            scrutinee(parse_quote! {
                #[dispatch(*)]
                fn poll(self: Pin<&mut Self>) -> usize {
                    match self {
                        Self::Empty => 0,
                    }
                }
            }),
            quote!(::core::pin::Pin::get_mut(self)).to_string(),
            "the pinned reference is unpinned"
        );
        assert_eq!(
            scrutinee(parse_quote! {
                #[dispatch(*)]
                fn peek(self: Pin<&Self>) -> usize {
                    match self {
                        Self::Empty => 0,
                    }
                }
            }),
            quote!(::core::pin::Pin::get_ref(self)).to_string()
        );
    }

    /// The closing arm of the kept `match` of `item`, spelled.
    fn closing_arm(item: ImplItemFn) -> String {
        let (function, spelled) = standing(item);
        let [Stmt::Expr(Expr::Match(closed), None)] = function.block.stmts.as_slice() else {
            panic!("one `match`: {spelled}");
        };
        let last = closed.arms.last().expect("an arm");
        quote!(#last).to_string()
    }

    #[test]
    fn the_closing_arm_reads_the_arguments_where_the_method_has_any() {
        assert_eq!(
            closing_arm(parse_quote! {
                #[dispatch(*)]
                fn write(&self, buf: &mut Vec<u8>, mut limit: usize) -> usize {
                    match self {
                        #[cfg(any())]
                        Self::Empty => limit + buf.len(),
                    }
                }
            }),
            quote! {
                #[allow(unreachable_patterns)]
                _ => {
                    let _ = (&buf, &limit,);
                    ::core::unreachable!()
                },
            }
            .to_string(),
            "every argument is read, in order, so none is unused where its arm is compiled out"
        );
        assert_eq!(
            closing_arm(parse_quote! {
                #[dispatch(*)]
                fn len(&self) -> usize {
                    match self {
                        Self::Empty => 0,
                    }
                }
            }),
            quote! {
                #[allow(unreachable_patterns)]
                _ => ::core::unreachable!(),
            }
            .to_string(),
            "a method without arguments has nothing to read"
        );
    }

    #[test]
    fn a_body_for_the_rest_or_one_that_holds_no_arms_stands_as_written() {
        for item in [
            parse_quote! {
                #[dispatch(Square)]
                fn len(&self) -> usize {
                    match self {
                        Self::Empty => 0,
                    }
                }
            },
            parse_quote! {
                #[dispatch(*)]
                fn len(&self) -> usize {
                    0
                }
            },
            parse_quote! {
                #[dispatch(*)]
                fn len(&self) -> usize {
                    match *self {
                        Self::Empty => 0,
                    }
                }
            },
            parse_quote! {
                #[dispatch(*, call = size)]
                fn len(&self) -> usize {
                    let zero = 0;
                    match self {
                        Self::Empty => zero,
                    }
                }
            },
        ] {
            let item: ImplItemFn = item;
            let mut written = item.clone();
            written
                .attrs
                .retain(|attribute| !is_dispatch_attribute(attribute));
            let (_, spelled) = standing(item);
            assert_eq!(spelled, quote!(#written).to_string());
        }
    }

    #[test]
    fn an_unmarked_method_with_arms_of_its_own_is_closed() {
        let method = marked(parse_quote! {
            #[dispatch(*, call = size)]
            fn len(&self) -> usize {
                match self {
                    Self::Empty => 0,
                }
            }
        });
        let Some(ImplItem::Fn(function)) = method.unmarked() else {
            panic!("the method as written");
        };
        let [Stmt::Expr(Expr::Match(closed), None)] = function.block.stmts.as_slice() else {
            panic!("one `match self`");
        };
        assert_eq!(closed.arms.len(), 2);
        assert!(function.attrs.is_empty(), "the mark is taken off");
    }
}
