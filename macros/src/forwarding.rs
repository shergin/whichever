//! The forwarding engine: how one method of a subject becomes a `match`
//! with one arm per target.
//!
//! A trait method and an inherent method differ in what each arm calls, in
//! which targets take part and in what the remaining arms do, and agree on
//! the rest: how the receiver is reached, how the arguments are bound, what
//! an arm does with the payload's result, and how the `match` becomes a
//! body. The agreeing parts live here as data, a [`MethodPlan`] read off the
//! signature, an [`Arm`] per target and the [`Forwarding`] that renders
//! them; `implement.rs` and `block.rs` fill the data in.
//!
//! Tokens the engine writes carry the context's `site`, the subject's or the
//! method's own span: a proc macro's call site, inside the carrier's
//! expansion, is the trait's attribute, which may be in another crate.
use proc_macro2::{Delimiter, Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote, quote_spanned, ToTokens};
use syn::{Attribute, Error, FnArg, Ident, Member, Pat, PatIdent, Path, Result, Signature, Token};

use crate::options::TraitSpec;
use crate::route::Route;
use crate::shape::{
    argument_kind, classify_receiver, classify_return, last_segment, turbofish, Access,
    ArgumentKind, Conversion, ReceiverKind, ReturnSite,
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
    /// The argument holds the same kind of target as the receiver: it is
    /// matched beside the receiver, on the same variant, and two different
    /// variants take the fallback arm. This is every trait method, where
    /// `Self` is the implementor, and an inherent method of a struct, whose
    /// two values always hold the same field.
    Dispatch,
    /// `Self` is the enum itself, in an inherent method of one: the
    /// argument is handed over as it is, and a payload method taking the
    /// enum is a legitimate target.
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
    /// The generic arguments a call spells out, when inference could not.
    turbofish: Option<TokenStream2>,
    return_model: Conversion,
    /// Whether the method returns `impl Future`, so that the `match` sits in
    /// one `async move` block and every arm is awaited inside it.
    pub(crate) future: bool,
    awaited: bool,
    unsafety: bool,
}

impl MethodPlan {
    /// Reads `signature`, which must have a receiver, and binds its
    /// arguments as `names` says. `spec` carries a trait's associated type
    /// overrides; an inherent method has none. `site` is what the return
    /// type's classification depends on besides the type.
    pub(crate) fn new(
        signature: &mut Signature,
        names: ArgumentNames,
        self_arguments: SelfArguments,
        spec: Option<&TraitSpec>,
        site: &ReturnSite,
        resolution: Option<Span>,
    ) -> Result<Self> {
        let (receiver, mut self_token) = match signature.receiver() {
            Some(receiver) => (classify_receiver(receiver)?, receiver.self_token),
            None => {
                return Err(Error::new_spanned(
                    &*signature,
                    "a forwarded method needs a receiver",
                ))
            }
        };
        // The receiver and the written arguments resolve where the
        // `mismatch` expression was written, so that an expression on the
        // enum sees names that came through the trait's carrier.
        let resolve = |mut ident: Ident| {
            if let Some(resolution) = resolution {
                ident.set_span(ident.span().resolved_at(resolution));
            }
            ident
        };
        if let Some(resolution) = resolution {
            self_token.span = self_token.span.resolved_at(resolution);
            if let Some(FnArg::Receiver(receiver)) = signature.inputs.first_mut() {
                receiver.self_token = self_token;
            }
        }
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
                    let ident = resolve(pattern.ident.clone());
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
        let (return_model, future) = classify_return(&signature.output, spec, site)?;
        Ok(Self {
            receiver,
            self_token,
            arguments,
            same_variant,
            turbofish: turbofish(signature),
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
    pub(crate) fn scrutinee(&self, site: Span) -> TokenStream2 {
        let receiver = receiver_scrutinee(self.receiver, &self.self_token, site);
        if self.same_variant.is_empty() {
            return receiver;
        }
        let arguments = self
            .same_variant
            .iter()
            .map(|(index, _)| &self.arguments[*index]);
        quote_spanned! { site => (#receiver, #(#arguments),*) }
    }

    fn same_binding(index: usize) -> Ident {
        hygienic(&format!("__same{index}"))
    }

    /// Returns the pattern binding `target`'s payload to `binding`, with the
    /// same-variant arguments' payloads bound beside it.
    fn pattern(&self, target: &Target, prefix: &Path, binding: &Ident) -> TokenStream2 {
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
    fn call_arguments(&self, target: &Target, site: Span) -> Vec<TokenStream2> {
        self.arguments
            .iter()
            .enumerate()
            .map(
                |(index, ident)| match self.same_variant.iter().find(|(same, _)| *same == index) {
                    Some((_, access)) => {
                        payload_expression(&target.route, *access, &Self::same_binding(index), site)
                    }
                    None => quote!(#ident),
                },
            )
            .collect()
    }

    /// Writes the arm for `target`: its pattern, the call, and what the arm
    /// does with the call's result.
    pub(crate) fn arm(&self, target: &Target, context: &ArmContext) -> Result<Arm> {
        let ArmContext {
            callee,
            method,
            subject,
            prefix,
            lift,
            site,
        } = context;
        let site = *site;
        let Target {
            ident: target_ident,
            payload,
            cfgs,
            route,
            ..
        } = target;
        // Under a lift the payload returns the plain value and the lift
        // wraps the rebuilt variant into the declared `Option` or `Result`.
        let conversion = if lift.is_some() {
            self.return_model.clone().unwrapped()
        } else {
            self.return_model.clone()
        };
        if !route.is_direct() && conversion.rewraps() {
            let message = if route.is_deref() {
                format!("`{method}` returns `Self`, which cannot be wrapped back into the dereferenced payload of `{target_ident}`; drop `deref` on it or {}", callee.by_hand())
            } else {
                format!("`{method}` returns `Self`, which cannot be wrapped back through the route `{}` of `{target_ident}`, whose other fields are gone; drop `via` on it or {}", route.spelled(), callee.by_hand())
            };
            return Err(Error::new_spanned(payload, message));
        }
        // What a method returns is a value, which `&mut self` cannot borrow
        // through; a field is a place, which it can.
        let exclusive = matches!(self.receiver, ReceiverKind::Value(Access::Exclusive))
            || matches!(self.receiver, ReceiverKind::Pinned(Access::Exclusive))
            || self
                .same_variant
                .iter()
                .any(|(_, access)| matches!(access, Access::Exclusive));
        if route.ends_in_a_method() && exclusive {
            return Err(Error::new_spanned(
                payload,
                format!("`{method}` takes `&mut self`, and the route `{}` of `{target_ident}` ends in a method call, whose result cannot be borrowed mutably; route to a field, or {}", route.spelled(), callee.by_hand()),
            ));
        }
        let inner = hygienic("__inner");
        let receiver = match (callee, self.receiver) {
            // Method-call syntax reaches through a pointer by itself, so
            // the route is followed from the binding as it is.
            (Callee::Method { .. }, ReceiverKind::Value(_)) => route.reach(&inner, false, site),
            (Callee::Place { .. }, _) => quote!(#inner),
            _ => receiver_expression(self.receiver, target, &inner, method, site)?,
        };
        let arguments = self.call_arguments(target, site);
        let turbofish = &self.turbofish;
        let mut value = match callee {
            Callee::Qualified { trait_path, method } => {
                // A field's or a method's type cannot be named; the call is
                // then qualified by the trait alone, which still keeps an
                // inherent method of the same name from taking over.
                let qualified = match target.target_type() {
                    Some(target_type) => quote_spanned! { site => <#target_type as #trait_path> },
                    None => quote_spanned! { site => <_ as #trait_path> },
                };
                quote_spanned! { site =>
                    #qualified::#method #turbofish (#receiver #(, #arguments)*)
                }
            }
            Callee::Method { name } => {
                quote_spanned! { site => #receiver.#name #turbofish (#(#arguments),*) }
            }
            // The arm is the target itself, or a field of it: a place,
            // borrowed as the declared return type asks.
            Callee::Place { field, borrow } => {
                let reached = route.reach(
                    &inner,
                    !matches!(self.receiver, ReceiverKind::Value(Access::Owned)),
                    site,
                );
                match (field, borrow) {
                    (None, _) => reached,
                    (Some(field), None) => quote_spanned! { site => #reached.#field },
                    (Some(field), Some(None)) => quote_spanned! { site => &#reached.#field },
                    (Some(field), Some(Some(_))) => quote_spanned! { site => &mut #reached.#field },
                }
            }
        };
        if self.unsafety {
            value = quote_spanned! { site => unsafe { #value } };
        }
        if self.awaited {
            value = quote_spanned! { site => #value.await };
        }
        let rebuild_error = || {
            Error::new_spanned(
                payload,
                format!("`{method}` returns `Self`, which cannot be built around the other fields of `{subject}`; {}", callee.rebuild_hint()),
            )
        };
        value = convert(value, &conversion, target, prefix, site, &rebuild_error)?;
        if let Some(lift) = lift {
            value = quote_spanned! { site => #lift(#value) };
        }
        Ok(Arm {
            cfgs: cfgs.clone(),
            pattern: self.pattern(target, prefix, &inner),
            value,
        })
    }

    /// Returns the arm for every pair of different variants, with `value`
    /// as its result. A same-variant argument the scrutinee moved, an owned
    /// or `&mut Self`, is bound again under its own name, so that a
    /// `mismatch` expression written on the trait method may use it.
    pub(crate) fn fallback(&self, value: TokenStream2) -> Arm {
        let moved = |access: &Access| !matches!(access, Access::Shared);
        if !self.same_variant.iter().any(|(_, access)| moved(access)) {
            return Arm {
                cfgs: vec![],
                pattern: quote!(_),
                value,
            };
        }
        let positions = self.same_variant.iter().map(|(index, access)| {
            if moved(access) {
                Self::same_binding(*index).into_token_stream()
            } else {
                quote!(_)
            }
        });
        let rebound = self
            .same_variant
            .iter()
            .filter(|(_, access)| moved(access))
            .map(|(index, _)| {
                let name = &self.arguments[*index];
                let binding = Self::same_binding(*index);
                quote!(let #name = #binding;)
            });
        Arm {
            cfgs: vec![],
            pattern: quote!((_, #(#positions),*)),
            value: quote!({ #(#rebound)* #value }),
        }
    }
}

/// What every arm of one method shares.
pub(crate) struct ArmContext<'a> {
    pub(crate) callee: Callee<'a>,
    /// The generated method's name, for messages.
    pub(crate) method: &'a Ident,
    /// The subject's name, for messages.
    pub(crate) subject: &'a Ident,
    /// What patterns are written on: the subject's name, or `Self`.
    pub(crate) prefix: Path,
    /// A function every arm's result is wrapped in, the inherent
    /// `with = path`.
    pub(crate) lift: Option<&'a Path>,
    /// The span the generated tokens carry.
    pub(crate) site: Span,
}

/// How an arm calls the payload's method, which is also which of the two
/// paths the method is on: a trait's, or an inherent block's.
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
    /// No call: the arm is the target itself, or with `field` a field of
    /// it, borrowed as `borrow` says, `None` by value, `Some(None)` by `&`
    /// and `Some(Some(mut))` by `&mut`.
    Place {
        field: Option<&'a Member>,
        borrow: Option<Option<Token![mut]>>,
    },
}

impl Callee<'_> {
    /// The way out a message offers when a method cannot be forwarded as
    /// written: the trait path points at an impl by hand, the inherent path
    /// at the method's body.
    pub(crate) fn by_hand(&self) -> &'static str {
        match self {
            Callee::Qualified { .. } => "implement the method by hand",
            Callee::Method { .. } | Callee::Place { .. } => "give it a body",
        }
    }

    /// The way out when a method returning `Self` cannot be rebuilt around
    /// the target.
    fn rebuild_hint(&self) -> &'static str {
        match self {
            Callee::Qualified { .. } => "write the method in a `#[whichever::dispatch] impl` block",
            Callee::Method { .. } | Callee::Place { .. } => "give it a body instead",
        }
    }
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
        prefix: &Path,
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

    /// The arm for everything else, with `value` as its result.
    pub(crate) fn rest(value: TokenStream2) -> Self {
        Arm {
            cfgs: vec![],
            pattern: quote!(_),
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
    /// The arm for everything the others leave: the mismatch of a trait
    /// method, or the fallback body of an inherent one.
    pub(crate) fallback: Option<Arm>,
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
        let mut body = quote_spanned! { span => match #scrutinee { #(#arms)* #fallback } };
        if *future {
            body = quote_spanned! { span => async move { #body } };
        }
        let mut group = Group::new(Delimiter::Brace, body);
        group.set_span(span);
        TokenTree::Group(group).into()
    }
}

/// Applies `conversion` to `value`, the payload's result, as a hand writes
/// it: the variant's constructor around a `Self`, `Into::into` around an
/// overridden associated type, each as the path a `.map(..)` or a
/// `.map_err(..)` takes, and a tuple taken apart and built again around
/// its converted elements.
fn convert(
    value: TokenStream2,
    conversion: &Conversion,
    target: &Target,
    prefix: &Path,
    site: Span,
    rebuild_error: &dyn Fn() -> Error,
) -> Result<TokenStream2> {
    // The path a `.map(..)` takes for a conversion of one value.
    let path = |conversion: &Conversion| -> Result<Option<TokenStream2>> {
        Ok(match conversion {
            Conversion::Keep => None,
            Conversion::Rewrap => Some(target.constructor(prefix).ok_or_else(rebuild_error)?),
            Conversion::Into => Some(quote_spanned! { site => ::core::convert::Into::into }),
            _ => unreachable!("a conversion under a wrapper is a path"),
        })
    };
    Ok(match conversion {
        Conversion::Keep => value,
        Conversion::Rewrap => target.rebuild(prefix, value).ok_or_else(rebuild_error)?,
        Conversion::Into => quote_spanned! { site => ::core::convert::Into::into(#value) },
        Conversion::Map(inner) => match path(inner)? {
            Some(path) => quote_spanned! { site => #value.map(#path) },
            None => value,
        },
        Conversion::MapErr(inner, error) => {
            let mut value = match path(inner)? {
                Some(path) => quote_spanned! { site => #value.map(#path) },
                None => value,
            };
            if let Some(path) = path(error)? {
                value = quote_spanned! { site => #value.map_err(#path) };
            }
            value
        }
        Conversion::Tuple(items) => {
            let bindings: Vec<Ident> = (0..items.len())
                .map(|index| hygienic(&format!("__element{index}")))
                .collect();
            let converted = items
                .iter()
                .zip(&bindings)
                .map(|(item, binding)| {
                    convert(quote!(#binding), item, target, prefix, site, rebuild_error)
                })
                .collect::<Result<Vec<_>>>()?;
            quote_spanned! { site => {
                let (#(#bindings,)*) = #value;
                (#(#converted,)*)
            } }
        }
    })
}

/// Returns the expression the generated `match` looks at for a receiver:
/// the value itself, the enum moved out of its box, or the pinned reference
/// unpinned.
fn receiver_scrutinee(kind: ReceiverKind, self_token: &Token![self], site: Span) -> TokenStream2 {
    match kind {
        ReceiverKind::Value(_) => quote!(#self_token),
        ReceiverKind::Boxed => quote_spanned!(site => *#self_token),
        ReceiverKind::Pinned(Access::Exclusive) => {
            quote_spanned!(site => ::core::pin::Pin::get_mut(#self_token))
        }
        ReceiverKind::Pinned(_) => quote_spanned!(site => ::core::pin::Pin::get_ref(#self_token)),
    }
}

/// Returns the expression handing the payload bound to `binding` to the
/// method as its receiver, or the error for a payload the receiver cannot
/// reach.
fn receiver_expression(
    kind: ReceiverKind,
    target: &Target,
    binding: &Ident,
    method: &Ident,
    site: Span,
) -> Result<TokenStream2> {
    match kind {
        ReceiverKind::Value(access) => Ok(payload_expression(&target.route, access, binding, site)),
        ReceiverKind::Boxed => {
            if !target.holds_box() {
                let ident = &target.ident;
                let payload = &target.payload;
                let spelled = payload.to_token_stream().to_string().replace(' ', "");
                // A payload that already is a `Box` only lacks `deref`.
                let advice = if last_segment(payload).is_some_and(|segment| segment.ident == "Box")
                {
                    format!("add `#[dispatch(deref)]` to `{ident}`")
                } else {
                    format!("hold `Box<{spelled}>` in a `#[dispatch(deref)]` variant")
                };
                return Err(Error::new_spanned(
                    payload,
                    format!("`{method}` takes `self: Box<Self>`, and `{ident}` holds a plain `{spelled}`; {advice}, or implement the method by hand"),
                ));
            }
            Ok(quote!(#binding))
        }
        ReceiverKind::Pinned(access) => {
            let payload = payload_expression(&target.route, access, binding, site);
            Ok(quote_spanned!(site => ::core::pin::Pin::new(#payload)))
        }
    }
}

/// Returns the expression that hands `binding`, bound to a variant's
/// payload, to the trait method: the payload itself, or the target the
/// route reaches, borrowed as the receiver is. A route ending in a method
/// call yields a value, which is handed over as it is.
fn payload_expression(route: &Route, access: Access, binding: &Ident, site: Span) -> TokenStream2 {
    if route.is_direct() {
        return quote!(#binding);
    }
    let reached = route.reach(binding, !matches!(access, Access::Owned), site);
    if route.ends_in_a_method() {
        return reached;
    }
    match access {
        Access::Shared => quote_spanned!(site => &#reached),
        Access::Exclusive => quote_spanned!(site => &mut #reached),
        Access::Owned => reached,
    }
}
