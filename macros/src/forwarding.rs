//! The forwarding engine: how one method of a subject becomes a `match`
//! with one arm per target.
//!
//! A trait method and an inherent method differ in what each arm calls, in
//! which targets take part and in what the remaining arms do, and agree on
//! the rest: how the receiver is reached, how the arguments are bound, what
//! an arm does with the payload's result, and how the `match` becomes a
//! body. The agreeing parts live here as data, a [`MethodPlan`] read off the
//! signature, an [`Arm`] per target and the [`Forwarding`] that renders
//! them; `implement.rs` and `inherent.rs` fill the data in.
use proc_macro2::{Delimiter, Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{Attribute, Error, FnArg, Ident, Pat, PatIdent, Path, Result, Signature, Token};

use crate::options::TraitSpec;
use crate::shape::{
    argument_kind, classify_receiver, classify_return, payload_expression, receiver_expression,
    scrutinee, Access, ArgumentKind, Conversion, ReceiverKind, ReturnModel, Value, Wrapper,
};
use crate::subject::{Target, Unforwardable};

/// Returns an identifier with `macro_rules`-style hygiene, which can never
/// collide with one the user wrote.
fn hygienic(name: &str) -> Ident {
    Ident::new(name, Span::mixed_site())
}

/// How the arguments after the receiver are bound in the generated
/// signature.
#[derive(Clone, Copy)]
pub(crate) enum ArgumentNames {
    /// The identifiers written, so that the generated signature reads like
    /// the trait's; a pattern that is not a plain identifier, which a trait
    /// method with a default body may have, gets a fresh hygienic
    /// `__argument<n>` instead.
    WrittenOrGenerated,
    /// The identifiers written, and an error for any other pattern: an
    /// inherent method's parameters are the user's to rename.
    Written,
}

/// Returns the pattern that binds `ident` and nothing else.
fn plain(ident: Ident) -> Pat {
    Pat::Ident(PatIdent {
        attrs: vec![],
        by_ref: None,
        mutability: None,
        ident,
        subpat: None,
    })
}

/// What an argument typed `Self`, `&Self` or `&mut Self` means.
#[derive(Clone, Copy)]
pub(crate) enum SelfArguments {
    /// `Self` is the implementor: the argument is matched on the same
    /// variant as the receiver, and two different variants take the
    /// fallback arm.
    Dispatch,
    /// `Self` is the subject itself: the argument is handed over as it is,
    /// and a payload method taking the subject is a legitimate target.
    PassThrough,
}

/// What is known about a method before any target is looked at.
pub(crate) struct MethodPlan {
    receiver: ReceiverKind,
    /// `self` resolves like a local variable, so the generated `match` uses
    /// the receiver's own token to share its hygiene.
    self_token: Token![self],
    /// The binding of every argument after the receiver, in order.
    arguments: Vec<Ident>,
    /// The arguments matched on the same variant as the receiver, by index.
    same_variant: Vec<(usize, Access)>,
    return_model: ReturnModel,
    /// Whether the method returns `impl Future`, so that the `match` sits in
    /// one `async move` block and every arm is awaited inside it.
    pub(crate) future: bool,
    awaited: bool,
    unsafety: bool,
}

impl MethodPlan {
    /// Reads `signature`, which must have a receiver, and binds its
    /// arguments as `names` says. `spec` carries a trait's associated type
    /// overrides; an inherent method has none.
    pub(crate) fn new(
        signature: &mut Signature,
        names: ArgumentNames,
        self_arguments: SelfArguments,
        spec: Option<&TraitSpec>,
    ) -> Result<Self> {
        let (receiver, self_token) = match signature.receiver() {
            Some(receiver) => (classify_receiver(receiver)?, receiver.self_token),
            None => {
                return Err(Error::new_spanned(
                    &*signature,
                    "a forwarded method needs a receiver",
                ))
            }
        };
        let mut arguments = vec![];
        let mut same_variant = vec![];
        for (index, input) in signature.inputs.iter_mut().skip(1).enumerate() {
            let FnArg::Typed(pat_type) = input else {
                unreachable!("the receiver is the first input")
            };
            let binding = match (&*pat_type.pat, names) {
                (Pat::Ident(pattern), ArgumentNames::Written) => pattern.ident.clone(),
                // The identifier alone: a `mut` or `ref` the trait wrote
                // would be unused or wrong around a plain forward.
                (Pat::Ident(pattern), ArgumentNames::WrittenOrGenerated)
                    if pattern.by_ref.is_none() && pattern.subpat.is_none() =>
                {
                    let ident = pattern.ident.clone();
                    *pat_type.pat = plain(ident.clone());
                    ident
                }
                (_, ArgumentNames::WrittenOrGenerated) => {
                    let ident = hygienic(&format!("__argument{index}"));
                    *pat_type.pat = plain(ident.clone());
                    ident
                }
                (other, ArgumentNames::Written) => {
                    return Err(Error::new_spanned(
                        other,
                        "a forwarded parameter must be a plain identifier",
                    ))
                }
            };
            if let SelfArguments::Dispatch = self_arguments {
                if let ArgumentKind::SameVariant(access) = argument_kind(&pat_type.ty)? {
                    same_variant.push((index, access));
                }
            }
            arguments.push(binding);
        }
        let (return_model, future) = classify_return(&signature.output, spec)?;
        Ok(Self {
            receiver,
            self_token,
            arguments,
            same_variant,
            return_model,
            future,
            awaited: signature.asyncness.is_some() || future,
            unsafety: signature.unsafety.is_some(),
        })
    }

    /// Returns whether a `Self` argument takes part in the `match`, so that
    /// two different variants need a fallback arm.
    pub(crate) fn has_same_variant(&self) -> bool {
        !self.same_variant.is_empty()
    }

    /// Returns the expression the `match` looks at: the receiver, with the
    /// same-variant arguments beside it in a tuple.
    pub(crate) fn scrutinee(&self) -> TokenStream2 {
        let receiver = scrutinee(self.receiver, &self.self_token);
        if self.same_variant.is_empty() {
            return receiver;
        }
        let arguments = self
            .same_variant
            .iter()
            .map(|(index, _)| &self.arguments[*index]);
        quote! { (#receiver, #(#arguments),*) }
    }

    fn same_binding(index: usize) -> Ident {
        hygienic(&format!("__same{index}"))
    }

    /// Returns the pattern binding `target`'s payload to `binding`, with the
    /// same-variant arguments' payloads bound beside it.
    fn pattern(&self, target: &Target, prefix: &TokenStream2, binding: &Ident) -> TokenStream2 {
        let own = target.pattern(prefix, binding);
        if self.same_variant.is_empty() {
            return own;
        }
        let same = self
            .same_variant
            .iter()
            .map(|(index, _)| target.pattern(prefix, &Self::same_binding(*index)));
        quote! { (#own, #(#same),*) }
    }

    /// Returns the arguments handed to `target`'s method: each binding as it
    /// is, or the payload of a same-variant argument.
    fn call_arguments(&self, target: &Target) -> Vec<TokenStream2> {
        self.arguments
            .iter()
            .enumerate()
            .map(
                |(index, ident)| match self.same_variant.iter().find(|(same, _)| *same == index) {
                    Some((_, access)) => {
                        payload_expression(target.deref, *access, &Self::same_binding(index))
                    }
                    None => quote!(#ident),
                },
            )
            .collect()
    }

    /// Writes the arm for `target`: its pattern, the call, and what the arm
    /// does with the call's result.
    pub(crate) fn arm(&self, target: &Target, arms: &Arms) -> Result<Arm> {
        let Arms {
            callee,
            method,
            subject,
            prefix,
            lift,
            hints,
        } = arms;
        let Target {
            ident: target_ident,
            payload,
            cfgs,
            deref,
            ..
        } = target;
        // Under a lift the payload returns the plain value and the lift
        // wraps the rebuilt variant into the declared `Option` or `Result`.
        let ReturnModel {
            wrapper,
            value: needs,
        } = self.return_model;
        let wrapper = if lift.is_some() {
            Wrapper::None
        } else {
            wrapper
        };
        if *deref && matches!(needs, Value::Rewrap) {
            return Err(Error::new_spanned(
                payload,
                format!("`{method}` returns `Self`, which cannot be wrapped back into the dereferenced payload of `{target_ident}`; drop `deref` on it or {}", hints.deref),
            ));
        }
        let inner = hygienic("__inner");
        let receiver = match (callee, self.receiver) {
            // Method-call syntax reaches through a `deref` payload by itself.
            (Callee::Method { .. }, ReceiverKind::Value(_)) => quote!(#inner),
            _ => receiver_expression(self.receiver, target, &inner, method)?,
        };
        let arguments = self.call_arguments(target);
        let mut value = match callee {
            Callee::Qualified { trait_path, method } => {
                let target_type = target.target_type();
                quote! { <#target_type as #trait_path>::#method(#receiver #(, #arguments)*) }
            }
            Callee::Method { name } => quote! { #receiver.#name(#(#arguments),*) },
        };
        if self.unsafety {
            value = quote! { unsafe { #value } };
        }
        if self.awaited {
            value = quote! { #value.await };
        }
        let rebuild_error = || {
            Error::new_spanned(
                payload,
                format!("`{method}` returns `Self`, which cannot be built around the other fields of `{subject}`; {}", hints.rebuild),
            )
        };
        value = match (wrapper, needs) {
            (_, Value::Plain) => value,
            (Wrapper::None, Value::Rewrap) => {
                target.rebuild(prefix, value).ok_or_else(rebuild_error)?
            }
            (Wrapper::None, Value::Into) => quote! { ::core::convert::Into::into(#value) },
            (_, Value::Rewrap) => {
                let constructor = target.constructor(prefix).ok_or_else(rebuild_error)?;
                quote! { #value.map(#constructor) }
            }
            (_, Value::Into) => quote! { #value.map(::core::convert::Into::into) },
        };
        if let Wrapper::Result {
            error: Conversion::Into,
        } = wrapper
        {
            value = quote! { #value.map_err(::core::convert::Into::into) };
        }
        if let Some(lift) = lift {
            value = quote! { #lift(#value) };
        }
        Ok(Arm {
            cfgs: cfgs.clone(),
            pattern: self.pattern(target, prefix, &inner),
            value,
        })
    }
}

/// What every arm of one method shares.
pub(crate) struct Arms<'a> {
    pub(crate) callee: Callee<'a>,
    /// The generated method's name, for messages.
    pub(crate) method: &'a Ident,
    /// The subject's name, for messages.
    pub(crate) subject: &'a Ident,
    /// What patterns are written on: the subject's name, or `Self`.
    pub(crate) prefix: TokenStream2,
    /// A function every arm's result is wrapped in, the inherent
    /// `with = path`.
    pub(crate) lift: Option<&'a Path>,
    pub(crate) hints: Hints,
}

/// How an arm calls the payload's method.
pub(crate) enum Callee<'a> {
    /// `<Target as Trait>::method(receiver, arguments..)`, so that an
    /// inherent method of the payload with the same name never takes over.
    Qualified {
        trait_path: &'a Path,
        method: &'a Ident,
    },
    /// `receiver.name(arguments..)`, for a method the payload has under
    /// that name.
    Method { name: &'a Ident },
}

/// The way out that a message offers when a method returning `Self` cannot
/// be rebuilt: the trait path points at a partial impl block, the inherent
/// path at the method's body.
pub(crate) struct Hints {
    pub(crate) deref: &'static str,
    pub(crate) rebuild: &'static str,
}

/// One arm of the generated `match`.
pub(crate) struct Arm {
    pub(crate) cfgs: Vec<Attribute>,
    pub(crate) pattern: TokenStream2,
    pub(crate) value: TokenStream2,
}

impl Arm {
    /// The arm for a variant no method can forward to, with `value` as its
    /// result; it carries the variant's `cfg` like any other arm.
    pub(crate) fn unforwardable(
        prefix: &TokenStream2,
        unforwardable: &Unforwardable,
        value: TokenStream2,
    ) -> Self {
        let ident = &unforwardable.ident;
        Arm {
            cfgs: unforwardable.cfgs.clone(),
            pattern: quote!(#prefix::#ident { .. }),
            value,
        }
    }
}

impl ToTokens for Arm {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Arm {
            cfgs,
            pattern,
            value,
        } = self;
        tokens.extend(quote! { #(#cfgs)* #pattern => #value, });
    }
}

/// One generated method body: a `match` over the subject.
pub(crate) struct Forwarding {
    pub(crate) scrutinee: TokenStream2,
    pub(crate) arms: Vec<Arm>,
    /// The expression of the `_ =>` arm: the mismatch result of a trait
    /// method, or the fallback body of an inherent one.
    pub(crate) fallback: Option<TokenStream2>,
    pub(crate) future: bool,
}

impl Forwarding {
    /// Renders the body: the `match`, inside an `async move` block when the
    /// method returns `impl Future` so that every arm is awaited where an
    /// `async fn` would await it, in braces carrying the method's own span,
    /// so that the item rustc points at as "defined here" is the method and
    /// not the attribute that generated it.
    pub(crate) fn body(&self, span: Span) -> TokenStream2 {
        let Forwarding {
            scrutinee,
            arms,
            fallback,
            future,
        } = self;
        let fallback = fallback.as_ref().map(|expr| quote! { _ => #expr, });
        let mut body = quote! { match #scrutinee { #(#arms)* #fallback } };
        if *future {
            body = quote! { async move { #body } };
        }
        let mut group = Group::new(Delimiter::Brace, body);
        group.set_span(span);
        TokenTree::Group(group).into()
    }
}
