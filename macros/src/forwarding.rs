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
//! expansion, is the trait's attribute, which may be in another crate. No
//! module that writes generated code uses a bare `quote!`; a unit test of
//! the crate reads the sources and says so.
use proc_macro2::{Delimiter, Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Attribute, Error, FnArg, Ident, Member, Pat, PatIdent, Path, Result, Signature, Token};

use crate::at;
use crate::options::TraitSpec;
use crate::route::Route;
use crate::shape::{
    argument_kind, classify_receiver, classify_return, is_self, last_segment, turbofish, Access,
    ArgumentKind, Conversion, FutureShape, PlaceBorrow, ReceiverKind, ReturnSite,
};
use crate::spelling;
use crate::subject::{Entry, Target, Unforwardable};

/// Returns an identifier with `macro_rules`-style hygiene, which can never
/// collide with one the user wrote, located at `site` as every other token
/// of an arm is: a binding left at the proc macro's call site would stretch
/// the span of the expression that holds it back to the attribute, and
/// rustc's errors with it.
fn hygienic(name: &str, site: Span) -> Ident {
    Ident::new(name, at(site))
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

/// Returns whether the signature's `where` clause has `Self: Sized`, the
/// bound a provided method takes to be left out of a trait object's
/// vtable, which a `dyn` payload therefore cannot satisfy.
fn requires_sized(signature: &Signature) -> bool {
    let Some(where_clause) = &signature.generics.where_clause else {
        return false;
    };
    where_clause.predicates.iter().any(|predicate| {
        let syn::WherePredicate::Type(predicate) = predicate else {
            return false;
        };
        is_self(&predicate.bounded_ty)
            && predicate.bounds.iter().any(|bound| match bound {
                syn::TypeParamBound::Trait(bound) => {
                    bound.maybe.is_none()
                        && bound
                            .path
                            .segments
                            .last()
                            .is_some_and(|segment| segment.ident == "Sized")
                }
                _ => false,
            })
    })
}

/// Returns whether `payload` is a `Box`, by the name its path ends in,
/// which is the one pointer a value is moved out of by dereferencing.
fn holds_a_box(payload: &syn::Type) -> bool {
    last_segment(payload).is_some_and(|segment| segment.ident == "Box")
}

/// Returns whether `payload` is a pointer that lends what it points at
/// shared and never exclusively: an `Rc` or an `Arc`, by the name its path
/// ends in, or a `&T`.
fn lends_only_shared(payload: &syn::Type) -> bool {
    match payload {
        syn::Type::Reference(reference) => reference.mutability.is_none(),
        other => last_segment(other)
            .is_some_and(|segment| segment.ident == "Rc" || segment.ident == "Arc"),
    }
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
    turbofish: Option<Vec<Ident>>,
    return_model: Conversion,
    /// The future the method returns, when the `match` sits in one `async
    /// move` block and every arm is awaited inside it.
    pub(crate) future: Option<FutureShape>,
    awaited: bool,
    unsafety: bool,
    /// Whether the method's `where` clause requires `Self: Sized`, which a
    /// trait object payload cannot satisfy.
    requires_sized: bool,
    /// Whether the `match` looks at the subject itself where the receiver
    /// is a reference, `match *self` with `ref` bindings.
    by_place: bool,
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
                (pattern, ArgumentNames::WrittenOrGenerated) => {
                    let ident = hygienic(&format!("__argument{index}"), pattern.span());
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
        let awaited = signature.asyncness.is_some() || future.is_some();
        Ok(Self {
            receiver,
            self_token,
            arguments,
            same_variant,
            turbofish: turbofish(signature),
            return_model,
            future,
            awaited,
            unsafety: matches!(signature.safety, syn::Safety::Unsafe(_)),
            requires_sized: requires_sized(signature),
            by_place: false,
        })
    }

    /// Says which entries the `match` is over. An enum whose every variant
    /// is under `#[cfg]` holds no variant at all in some build, and a
    /// reference to it still counts as inhabited there, so `match self {}`
    /// is not exhaustive while `match *self {}` is. A hand writes the
    /// second over such an enum, `match *self { Variant(ref inner) => .. }`,
    /// and so does the engine, for a reference receiver. A `Self` argument
    /// is matched beside the receiver in a tuple, which has the fallback
    /// arm in every build and stays as it is.
    pub(crate) fn over(mut self, entries: &[Entry]) -> Self {
        let always_present = entries.iter().any(|entry| match entry {
            Entry::Target(target) => target.cfgs.is_empty(),
            Entry::Unforwardable(unforwardable) => unforwardable.cfgs.is_empty(),
        });
        self.by_place = self.binds_references() && self.same_variant.is_empty() && !always_present;
        self
    }

    /// Returns whether a pattern over the receiver binds references: the
    /// receiver is one, plain or pinned, where `self` and `self: Box<Self>`
    /// are matched by value and bind the payload itself.
    fn binds_references(&self) -> bool {
        matches!(
            self.receiver,
            ReceiverKind::Value(Access::Shared | Access::Exclusive) | ReceiverKind::Pinned(_)
        )
    }

    /// Returns the first `Self` argument matched beside the receiver that
    /// the method takes as `access` says, for a sentence that names what
    /// the method takes.
    fn taken(&self, access: Access) -> Option<&Ident> {
        self.same_variant
            .iter()
            .find(|(_, taken)| *taken == access)
            .map(|(index, _)| &self.arguments[*index])
    }

    /// Returns whether a `Self` argument takes part in the `match`, so that
    /// two different variants need a fallback arm.
    pub(crate) fn has_same_variant(&self) -> bool {
        !self.same_variant.is_empty()
    }

    /// Returns the result of an arm that only keeps the `match` exhaustive
    /// beside a variant's own error. It reads the arguments first: in a
    /// build where no other arm is left nothing else uses them, and the
    /// lint about them would stand beside that error. The arm is under its
    /// variant's `cfg`, so a build without the variant has none of this.
    pub(crate) fn standing_in(&self, site: Span) -> TokenStream2 {
        let arguments = &self.arguments;
        if arguments.is_empty() {
            return quote_spanned!(site => ::core::unreachable!());
        }
        quote_spanned!(site => {
            let _ = (#(&#arguments,)*);
            ::core::unreachable!()
        })
    }

    /// Returns `arm`, whose pattern matches the receiver alone, as an arm
    /// of this method's `match`: where `Self` arguments are matched beside
    /// the receiver the scrutinee is a tuple, and the pattern is the
    /// receiver's beside whatever the arguments hold.
    pub(crate) fn beside_arguments(&self, arm: Arm) -> Arm {
        if self.same_variant.is_empty() {
            return arm;
        }
        let Arm {
            cfgs,
            pattern,
            value,
            site,
        } = arm;
        Arm {
            cfgs,
            pattern: quote_spanned!(site => (#pattern, ..)),
            value,
            site,
        }
    }

    /// Writes the arm for `target`, or, where the method cannot be
    /// forwarded to it, the arm that says so: the variant whatever it
    /// holds, with the sentence as its result, under the variant's own
    /// attributes. A hand's arm sits under its variant's `cfg`, so a build
    /// that compiles the variant out has neither the arm nor the sentence
    /// and forwards the rest; and each variant the method cannot reach is
    /// named, not the first alone.
    pub(crate) fn arm_or_sentence(&self, target: &Target, context: &ArmContext) -> Arm {
        let error = match self.arm(target, context) {
            Ok(arm) => return arm,
            Err(error) => error,
        };
        let site = context.site;
        let sentence = error.to_compile_error();
        self.beside_arguments(Arm {
            cfgs: target.arm_attributes(site),
            pattern: target.whole(&context.prefix, site),
            value: quote_spanned!(site => { #sentence }),
            site,
        })
    }

    /// Returns whether the method requires `Self: Sized`, so that a trait
    /// object payload cannot take it.
    pub(crate) fn requires_sized(&self) -> bool {
        self.requires_sized
    }

    /// Returns the expression the `match` looks at: the receiver, with the
    /// same-variant arguments beside it in a tuple.
    pub(crate) fn scrutinee(&self, site: Span) -> TokenStream2 {
        let receiver = receiver_scrutinee(self.receiver, &self.self_token, site);
        if self.by_place {
            return quote_spanned! { site => *#receiver };
        }
        if self.same_variant.is_empty() {
            return receiver;
        }
        let arguments = self
            .same_variant
            .iter()
            .map(|(index, _)| &self.arguments[*index]);
        quote_spanned! { site => (#receiver, #(#arguments),*) }
    }

    fn same_binding(index: usize, site: Span) -> Ident {
        hygienic(&format!("__same{index}"), site)
    }

    /// Returns the pattern binding `target`'s payload to `binding`, with the
    /// same-variant arguments' payloads bound beside it. Where the `match`
    /// looks at the subject itself, the binding borrows the payload as the
    /// receiver borrows the subject, so it is the reference it is anywhere
    /// else.
    fn pattern(&self, target: &Target, prefix: &Path, binding: &Ident, site: Span) -> TokenStream2 {
        if self.by_place {
            let exclusive = matches!(
                self.receiver,
                ReceiverKind::Value(Access::Exclusive) | ReceiverKind::Pinned(Access::Exclusive)
            );
            let borrowed = if exclusive {
                quote_spanned! { site => ref mut #binding }
            } else {
                quote_spanned! { site => ref #binding }
            };
            return target.pattern(prefix, &borrowed, site);
        }
        let own = target.pattern(prefix, binding, site);
        if self.same_variant.is_empty() {
            return own;
        }
        let same = self
            .same_variant
            .iter()
            .map(|(index, _)| target.pattern(prefix, &Self::same_binding(*index, site), site));
        quote_spanned! { site => (#own, #(#same),*) }
    }

    /// Returns the arguments handed to `target`'s method: each binding as it
    /// is, or the payload of a same-variant argument.
    fn call_arguments(&self, target: &Target, site: Span) -> Vec<TokenStream2> {
        self.arguments
            .iter()
            .enumerate()
            .map(
                |(index, ident)| match self.same_variant.iter().find(|(same, _)| *same == index) {
                    Some((_, access)) => payload_expression(
                        &target.route,
                        *access,
                        &Self::same_binding(index, site),
                        site,
                    ),
                    None => quote_spanned!(site => #ident),
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
            has_body: _,
            site,
        } = context;
        let site = *site;
        let Target {
            ident: target_ident,
            payload,
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
                format!("`{method}` returns `Self`, which cannot be wrapped back into the dereferenced payload of `{target_ident}`; drop `deref` on it or {}", context.way_out(target_ident))
            } else {
                format!("`{method}` returns `Self`, which cannot be wrapped back through the route `{}` of `{target_ident}`, whose other fields are gone; drop `via` on it or {}", route.spelled(), context.way_out(target_ident))
            };
            return Err(Error::new_spanned(payload, message));
        }
        // What a method returns is a value, which `&mut self` cannot borrow
        // through; a field is a place, which it can.
        let exclusive = match self.receiver {
            ReceiverKind::Value(Access::Exclusive) => Some(String::from("`&mut self`")),
            ReceiverKind::Pinned(Access::Exclusive) => Some(String::from("`self: Pin<&mut Self>`")),
            ReceiverKind::Value(Access::Shared | Access::Owned)
            | ReceiverKind::Pinned(Access::Shared | Access::Owned)
            | ReceiverKind::Boxed => self
                .taken(Access::Exclusive)
                .map(|argument| format!("`{argument}: &mut Self`")),
        };
        if let Some(taken) = exclusive.as_ref().filter(|_| route.ends_in_a_method()) {
            return Err(Error::new_spanned(
                payload,
                format!("`{method}` takes {taken}, and the route `{}` of `{target_ident}` ends in a method call, whose result cannot be borrowed mutably; route to a field, or {}", route.spelled(), context.way_out(target_ident)),
            ));
        }
        // An `Rc`, an `Arc` and a `&T` lend what they point at shared and
        // never exclusively. A call borrows its target as the method takes
        // it; an arm that is a place borrows it as the return type asks.
        // Any other pointer is rustc's to judge.
        let borrows_mutably = match callee {
            Callee::Place { borrow, .. } => matches!(borrow, PlaceBorrow::Reference(Some(_)))
                .then(|| String::from("returns a `&mut`")),
            Callee::Qualified { .. } | Callee::Method { .. } => {
                exclusive.map(|taken| format!("takes {taken}"))
            }
        };
        if let Some(does) =
            borrows_mutably.filter(|_| route.starts_with_deref() && lends_only_shared(payload))
        {
            let spelled = spelling::spelled(payload);
            return Err(Error::new_spanned(
                payload,
                format!("`{method}` {does}, and `{target_ident}` dereferences a `{spelled}`, which cannot be borrowed mutably; hold a `Box` or a `&mut`, or {}", context.way_out(target_ident)),
            ));
        }
        // A value is moved out of a `Box` by dereferencing it, and out of
        // nothing else: a reference or a shared pointer keeps what it points
        // at. Under `self: Box<Self>` the enum is matched by value as well:
        // an arm that is a call hands the payload's own box over, which the
        // receiver's expression checks, and an arm that is a place moves
        // its target out unless the return type borrows it.
        // By value as well, an arm that is a place moves its target out
        // unless the return type borrows it: `&*inner` on a `&'a T` payload
        // is the reference the variant holds, and moves nothing.
        let moves_the_place = !matches!(
            callee,
            Callee::Place {
                borrow: PlaceBorrow::Reference(_),
                ..
            }
        );
        let moved = match self.receiver {
            ReceiverKind::Boxed if matches!(callee, Callee::Place { .. }) && moves_the_place => {
                Some(String::from("`self: Box<Self>`"))
            }
            ReceiverKind::Value(Access::Owned) if moves_the_place => {
                Some(String::from("`self` by value"))
            }
            ReceiverKind::Value(_) | ReceiverKind::Pinned(_) | ReceiverKind::Boxed => self
                .taken(Access::Owned)
                .map(|argument| format!("`{argument}` by value")),
        };
        if let Some(taken) = moved.filter(|_| route.starts_with_deref() && !holds_a_box(payload)) {
            let spelled = spelling::spelled(payload);
            return Err(Error::new_spanned(
                payload,
                format!("`{method}` takes {taken}, and `{target_ident}` dereferences a `{spelled}`, which a value cannot be moved out of; hold a `Box`, or {}", context.way_out(target_ident)),
            ));
        }
        let inner = hygienic("__inner", site);
        let receiver = match (callee, self.receiver) {
            // The route is followed through the binding's reference, as for
            // a qualified call, so that `deref` calls the method on what the
            // payload points at and never on the pointer, whose own method
            // of the same name would otherwise win; method-call syntax then
            // borrows the target as the method's receiver asks.
            (Callee::Method { .. }, ReceiverKind::Value(access)) => {
                route.reach_for_postfix(&inner, !matches!(access, Access::Owned), site)
            }
            (Callee::Place { .. }, _) => quote_spanned!(site => #inner),
            _ => receiver_expression(self.receiver, target, &inner, context)?,
        };
        let arguments = self.call_arguments(target, site);
        // The angle brackets are the arm's own tokens: written where the
        // trait's method is, they would stretch the call's span from there
        // to the subject, across every attribute between.
        let turbofish = self
            .turbofish
            .as_ref()
            .map(|parameters| quote_spanned!(site => ::<#(#parameters),*>));
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
                // The route starts at the binding, which is a reference to
                // the payload only where the pattern binds one: `self` and
                // `self: Box<Self>` are matched by value, and the route
                // starts at the payload itself.
                let through_reference = self.binds_references();
                let place = match field {
                    Some(field) => {
                        let reached = route.reach_for_postfix(&inner, through_reference, site);
                        quote_spanned! { site => #reached.#field }
                    }
                    None => route.reach(&inner, through_reference, site),
                };
                // A place reached through a field or a route is borrowed as
                // the return type asks, so `via = .name, value` and `field
                // = name` are one arm. The target the pattern binds, a
                // payload, a marked field or a struct's field, is asked
                // the same way: under a reference receiver its binding is
                // a reference to it, which is the arm where the return
                // type asks for one and is read through, `*inner`, where
                // it asks for a value. A lift whose parameter is unread
                // takes the binding as it stands, and an awaited arm
                // awaits it, since the return type then speaks of the
                // future's output.
                if field.is_none() && route.is_direct() {
                    let read =
                        through_reference && !self.awaited && matches!(borrow, PlaceBorrow::Value);
                    if read {
                        quote_spanned! { site => *#place }
                    } else {
                        place
                    }
                } else {
                    match borrow {
                        PlaceBorrow::Value | PlaceBorrow::Unread => place,
                        PlaceBorrow::Reference(None) => quote_spanned! { site => &#place },
                        PlaceBorrow::Reference(Some(_)) => {
                            quote_spanned! { site => &mut #place }
                        }
                    }
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
                format!("`{method}` returns `Self`, which cannot be built around the other fields of `{subject}`; {}", context.rebuild_hint(target_ident)),
            )
        };
        value = convert(value, &conversion, target, prefix, site, &rebuild_error)?;
        if let Some(lift) = lift {
            value = quote_spanned! { site => #lift(#value) };
        }
        Ok(Arm {
            cfgs: target.arm_attributes(site),
            pattern: self.pattern(target, prefix, &inner, site),
            value,
            site,
        })
    }

    /// Returns the arm for every pair of different variants, with `value`
    /// as its result. A same-variant argument the scrutinee moved, an owned
    /// or `&mut Self`, is bound again under its own name where `value`
    /// names it, so that a `mismatch` expression may use it; one the
    /// expression does not name is left in the scrutinee, as a hand leaves
    /// it, and no unused binding is written.
    pub(crate) fn fallback(&self, value: TokenStream2, site: Span) -> Arm {
        let rebinds = |(index, access): &(usize, Access)| {
            !matches!(access, Access::Shared) && names(&value, &self.arguments[*index])
        };
        if !self.same_variant.iter().any(rebinds) {
            return Arm::rest(value, site);
        }
        let positions = self.same_variant.iter().map(|same| {
            if rebinds(same) {
                Self::same_binding(same.0, site).into_token_stream()
            } else {
                quote_spanned!(site => _)
            }
        });
        let rebound = self
            .same_variant
            .iter()
            .filter(|same| rebinds(same))
            .map(|(index, _)| {
                let name = &self.arguments[*index];
                let binding = Self::same_binding(*index, site);
                quote_spanned!(site => let #name = #binding;)
            });
        Arm {
            cfgs: vec![],
            pattern: quote_spanned!(site => (_, #(#positions),*)),
            value: quote_spanned!(site => { #(#rebound)* #value }),
            site,
        }
    }
}

/// Returns whether `tokens` name `ident`: as an identifier at any depth,
/// or inside a literal, where a format string captures it.
fn names(tokens: &TokenStream2, ident: &Ident) -> bool {
    let name = ident.to_string();
    tokens.clone().into_iter().any(|tree| match tree {
        TokenTree::Ident(found) => found == name,
        TokenTree::Group(group) => names(&group.stream(), ident),
        TokenTree::Literal(literal) => literal.to_string().contains(&name),
        TokenTree::Punct(_) => false,
    })
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
    /// Whether the method names its variants and has a body for the rest,
    /// which is then the way out of a variant it cannot be forwarded to.
    pub(crate) has_body: bool,
    /// The span the generated tokens carry.
    pub(crate) site: Span,
}

impl ArmContext<'_> {
    /// The way out a sentence about `variant` offers: the body, where the
    /// method has one for the variants it does not name, and otherwise
    /// what the callee's path offers a method that cannot be forwarded.
    pub(crate) fn way_out(&self, variant: &Ident) -> String {
        if self.has_body {
            return format!("leave `{variant}` to the body");
        }
        String::from(self.callee.by_hand())
    }

    /// The way out when the value cannot be built around `variant`'s
    /// payload again.
    fn rebuild_hint(&self, variant: &Ident) -> String {
        if self.has_body {
            return format!("leave `{variant}` to the body");
        }
        String::from(self.callee.rebuild_hint())
    }
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
    /// it, handed over as `borrow` says: by `&` or `&mut`, by value, or as
    /// it stands where what takes it is unread.
    Place {
        field: Option<&'a Member>,
        borrow: PlaceBorrow,
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
    /// The span of the arm's own tokens, the arrow and the comma.
    pub(crate) site: Span,
}

impl Arm {
    /// The arm for a variant no method can forward to, with `value` as its
    /// result; it carries the variant's `cfg` like any other arm.
    pub(crate) fn unforwardable(
        prefix: &Path,
        unforwardable: &Unforwardable,
        value: TokenStream2,
        site: Span,
    ) -> Self {
        let ident = &unforwardable.ident;
        Arm {
            cfgs: unforwardable.arm_attributes(site),
            pattern: quote_spanned!(site => #prefix::#ident { .. }),
            value,
            site,
        }
    }

    /// The arm for everything else, with `value` as its result.
    pub(crate) fn rest(value: TokenStream2, site: Span) -> Self {
        Arm {
            cfgs: vec![],
            pattern: quote_spanned!(site => _),
            value,
            site,
        }
    }
}

impl ToTokens for Arm {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Arm {
            cfgs,
            pattern,
            value,
            site,
        } = self;
        tokens.extend(quote_spanned! { *site => #(#cfgs)* #pattern => #value, });
    }
}

/// One generated method body: a `match` over the subject.
pub(crate) struct Forwarding {
    pub(crate) scrutinee: TokenStream2,
    pub(crate) arms: Vec<Arm>,
    /// The arm for everything the others leave: the mismatch of a trait
    /// method, or the fallback body of an inherent one.
    pub(crate) fallback: Option<Arm>,
    /// The future the method returns, when every arm is awaited inside one
    /// `async move` block: an opaque one, or a boxed one pinned again with
    /// the `Box` the signature names.
    pub(crate) future: Option<FutureShape>,
}

impl Forwarding {
    /// Renders the body: the `match`, inside an `async move` block when the
    /// method returns a future the arms are awaited in, so that every arm
    /// is awaited where an `async fn` would await it, in braces carrying the
    /// method's own span, so that the item rustc points at as "defined here"
    /// is the method and not the attribute that generated it.
    pub(crate) fn body(&self, span: Span) -> TokenStream2 {
        let Forwarding {
            scrutinee,
            arms,
            fallback,
            future,
        } = self;
        let mut body = quote_spanned! { span => match #scrutinee { #(#arms)* #fallback } };
        match future {
            None => {}
            Some(FutureShape::Opaque) => {
                body = quote_spanned! { span => async move { #body } };
            }
            Some(FutureShape::Boxed(path)) => {
                body = quote_spanned! { span => #path::pin(async move { #body }) };
            }
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
            Conversion::Rewrap => Some(target.constructor(prefix, site).ok_or_else(rebuild_error)?),
            Conversion::Into => Some(quote_spanned! { site => ::core::convert::Into::into }),
            _ => unreachable!("a conversion under a wrapper is a path"),
        })
    };
    Ok(match conversion {
        Conversion::Keep => value,
        Conversion::Rewrap => target
            .rebuild(prefix, value, site)
            .ok_or_else(rebuild_error)?,
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
                .map(|index| hygienic(&format!("__element{index}"), site))
                .collect();
            let converted = items
                .iter()
                .zip(&bindings)
                .map(|(item, binding)| {
                    let binding = quote_spanned!(site => #binding);
                    convert(binding, item, target, prefix, site, rebuild_error)
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
        ReceiverKind::Value(_) => quote_spanned!(site => #self_token),
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
    context: &ArmContext,
) -> Result<TokenStream2> {
    let ArmContext { method, site, .. } = context;
    let site = *site;
    match kind {
        ReceiverKind::Value(access) => Ok(payload_expression(&target.route, access, binding, site)),
        ReceiverKind::Boxed => {
            if !target.holds_box() {
                let ident = &target.ident;
                let payload = &target.payload;
                let spelled = spelling::spelled(payload);
                // A `deref` variant over another pointer has no box to hand
                // over, and boxing the pointer would not make one of what
                // it points at.
                if target.route.starts_with_deref() {
                    return Err(Error::new_spanned(
                        payload,
                        format!("`{method}` takes `self: Box<Self>`, and `{ident}` dereferences a `{spelled}`, which is no `Box` to hand over; hold a `Box`, or {}", context.way_out(ident)),
                    ));
                }
                // A payload that already is a `Box` only lacks `deref`.
                let advice = if last_segment(payload).is_some_and(|segment| segment.ident == "Box")
                {
                    format!("add `#[dispatch(deref)]` to `{ident}`")
                } else {
                    format!("hold `Box<{spelled}>` in a `#[dispatch(deref)]` variant")
                };
                return Err(Error::new_spanned(
                    payload,
                    format!("`{method}` takes `self: Box<Self>`, and `{ident}` holds a plain `{spelled}`; {advice}, or {}", context.way_out(ident)),
                ));
            }
            Ok(quote_spanned!(site => #binding))
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
        return quote_spanned!(site => #binding);
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
