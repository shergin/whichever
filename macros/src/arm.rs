//! One arm of a forwarding `match`: whether a method can be forwarded to a
//! target, and if it can, how the arm reaches the target, what it does
//! there and what it does with the result.
//!
//! The judgement and the writing are apart. [`MethodPlan::refusal`] says
//! why a target cannot take the method, as a value, and
//! [`MethodPlan::arm`] writes the arm for a target that can, or the arm
//! that holds the refusal's sentence.
use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::{Attribute, Error, Ident, Member, Path, Signature, Token};

use crate::forwarding::MethodPlan;
use crate::refusal::{MutableBorrow, Refusal, Taken, WayOut};
use crate::route::Route;
use crate::shape::{Access, Borrow, Conversion, PlaceBorrow, Pointer, ReceiverKind};
use crate::site::Site;
use crate::subject::{Entry, Target};

/// Returns an identifier with `macro_rules`-style hygiene, which can never
/// collide with one the user wrote, located at `site` as every other token
/// of an arm is: a binding left at the proc macro's call site would stretch
/// the span of the expression that holds it back to the attribute, and
/// rustc's errors with it.
pub(crate) fn hygienic(name: &str, site: Site) -> Ident {
    Ident::new(name, *site)
}

/// What an arm does with its target.
pub(crate) enum ArmKind<'a> {
    /// `<Target as Trait>::method(receiver, arguments..)`, so that an
    /// inherent method of the payload with the same name never takes over.
    QualifiedCall {
        trait_path: &'a Path,
        method: &'a Ident,
    },
    /// `receiver.name(arguments..)`, for a method the payload has under
    /// that name.
    MethodCall { name: &'a Ident },
    /// No call: the arm is the target itself, or with `field` a field of
    /// it, handed over as `borrow` says: by `&` or `&mut`, by value, or as
    /// it stands where what takes it is unread.
    Place {
        field: Option<&'a Member>,
        borrow: PlaceBorrow,
    },
}

/// The trait's side of a trait method, which a sentence about the method
/// points at.
#[derive(Clone, Copy)]
pub(crate) struct Declared<'a> {
    /// The signature as the trait declares it.
    pub(crate) signature: &'a Signature,
    /// The partial impl block a sentence points at, spelled out.
    pub(crate) block_header: &'a str,
}

/// What every arm of one method shares.
pub(crate) struct ArmContext<'a> {
    pub(crate) kind: ArmKind<'a>,
    /// The generated method's name, for messages.
    pub(crate) method: &'a Ident,
    /// The subject's name, for messages.
    pub(crate) subject: &'a Ident,
    /// What patterns are written on: the subject's name, or `Self`.
    pub(crate) prefix: Path,
    /// A function every arm's result is wrapped in, the inherent
    /// `with = path`.
    pub(crate) lift: Option<&'a Path>,
    /// The ways out a sentence about the method offers.
    pub(crate) way_out: WayOut,
    /// The trait's declaration of the method; an inherent method has none.
    pub(crate) declared: Option<Declared<'a>>,
    /// The span the generated tokens carry.
    pub(crate) site: Site,
}

/// One arm of the generated `match`.
pub(crate) struct Arm {
    /// The attributes of the variant the arm is for: its `#[cfg]`, and
    /// `#[allow(deprecated)]` when it is deprecated.
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) pattern: TokenStream2,
    pub(crate) value: TokenStream2,
    /// The span of the arm's own tokens, the arrow and the comma.
    pub(crate) site: Site,
}

impl Arm {
    /// The arm for a variant no method can forward to, with `value` as its
    /// result; it carries the variant's `cfg` like any other arm.
    pub(crate) fn unforwardable(
        prefix: &Path,
        entry: &Entry,
        value: TokenStream2,
        site: Site,
    ) -> Self {
        Arm {
            attributes: entry.arm_attributes(site),
            pattern: entry.whole(prefix, site),
            value,
            site,
        }
    }

    /// The arm for everything else, with `value` as its result.
    pub(crate) fn rest(value: TokenStream2, site: Site) -> Self {
        Arm {
            attributes: vec![],
            pattern: quote_spanned!(*site => _),
            value,
            site,
        }
    }
}

impl ToTokens for Arm {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Arm {
            attributes,
            pattern,
            value,
            site,
        } = self;
        tokens.extend(quote_spanned! { **site => #(#attributes)* #pattern => #value, });
    }
}

impl MethodPlan {
    /// Writes the arm for `target`, or, where the method cannot be
    /// forwarded to it, the arm that says so: the variant whatever it
    /// holds, with the sentence as its result, under the variant's own
    /// attributes. A hand's arm sits under its variant's `cfg`, so a build
    /// that compiles the variant out has neither the arm nor the sentence
    /// and forwards the rest; and each variant the method cannot reach is
    /// named, not the first alone.
    pub(crate) fn arm(&self, target: &Target, context: &ArmContext) -> Arm {
        match self.refusal(target, context) {
            Some(refusal) => {
                let error = refusal.error(context.method, target, context.subject, context.way_out);
                self.sentence_arm(target, context, error)
            }
            None => self.forward(target, context),
        }
    }

    /// Returns why the method cannot be forwarded to `target`, when it
    /// cannot. The first reason that holds is the one given.
    pub(crate) fn refusal<'a>(
        &'a self,
        target: &Target,
        context: &ArmContext<'a>,
    ) -> Option<Refusal<'a>> {
        let Target { payload, route, .. } = *target;
        if let Some(declared) = context.declared {
            if self.requires_sized && target.is_trait_object() {
                return Some(Refusal::TraitObject {
                    declared: declared.signature,
                    block_header: declared.block_header,
                });
            }
        }
        let conversion = self.conversion_under(context.lift);
        if !route.is_direct() && conversion.rewraps() {
            return Some(Refusal::RewrapThroughRoute);
        }
        // What a method returns is a value, which `&mut self` cannot borrow
        // through; a field is a place, which it can.
        let exclusive = match self.receiver {
            ReceiverKind::Value(Access::Exclusive) => Some(Taken::MutSelf),
            ReceiverKind::Pinned(Borrow::Exclusive) => Some(Taken::PinnedMutSelf),
            ReceiverKind::Value(Access::Shared | Access::Owned)
            | ReceiverKind::Pinned(Borrow::Shared)
            | ReceiverKind::Boxed => self.taken(Access::Exclusive).map(Taken::MutArgument),
        };
        if let Some(taken) = exclusive.filter(|_| route.ends_in_a_method()) {
            return Some(Refusal::MethodRouteBorrowedMutably(taken));
        }
        // An `Rc`, an `Arc` and a `&T` lend what they point at shared and
        // never exclusively. A call borrows its target as the method takes
        // it; an arm that is a place borrows it as the return type asks.
        // Any other pointer is rustc's to judge.
        let pointer = Pointer::of(payload);
        let borrows_mutably = match &context.kind {
            ArmKind::Place { borrow, .. } => {
                matches!(borrow, PlaceBorrow::Reference(Borrow::Exclusive))
                    .then_some(MutableBorrow::Returns)
            }
            ArmKind::QualifiedCall { .. } | ArmKind::MethodCall { .. } => {
                exclusive.map(MutableBorrow::Takes)
            }
        };
        if let Some(does) =
            borrows_mutably.filter(|_| route.starts_with_deref() && pointer == Pointer::Shared)
        {
            return Some(Refusal::SharedPointerBorrowedMutably(does));
        }
        // A value is moved out of a `Box` by dereferencing it, and out of
        // nothing else: a reference or a shared pointer keeps what it points
        // at. Under `self: Box<Self>` the enum is matched by value as well:
        // an arm that is a call hands the payload's own box over, which is
        // asked for below, and an arm that is a place moves its target out
        // unless the return type borrows it. By value as well, an arm that
        // is a place moves its target out unless the return type borrows
        // it: `&*inner` on a `&'a T` payload is the reference the variant
        // holds, and moves nothing.
        let is_place = matches!(context.kind, ArmKind::Place { .. });
        let moves_the_place = !matches!(
            context.kind,
            ArmKind::Place {
                borrow: PlaceBorrow::Reference(_),
                ..
            }
        );
        let moved = match self.receiver {
            ReceiverKind::Boxed if is_place && moves_the_place => Some(Taken::BoxedSelf),
            ReceiverKind::Value(Access::Owned) if moves_the_place => Some(Taken::SelfByValue),
            ReceiverKind::Value(_) | ReceiverKind::Pinned(_) | ReceiverKind::Boxed => {
                self.taken(Access::Owned).map(Taken::ArgumentByValue)
            }
        };
        if let Some(taken) = moved.filter(|_| route.starts_with_deref() && pointer != Pointer::Box)
        {
            return Some(Refusal::MovedOutOfPointer(taken));
        }
        // A call under `self: Box<Self>` hands the payload's own box over.
        if matches!(self.receiver, ReceiverKind::Boxed) && !is_place && !target.holds_box() {
            return Some(Refusal::NoBoxToHandOver);
        }
        let rebuilds = conversion.rebuilds();
        if (rebuilds.whole && !target.is_rebuilt())
            || (rebuilds.by_path && !target.is_built_by_a_path())
        {
            return Some(Refusal::NotRebuilt);
        }
        None
    }

    /// Returns what an arm does with the payload's result. Under a lift
    /// the payload returns the plain value and the lift wraps the rebuilt
    /// variant into the declared `Option` or `Result`.
    fn conversion_under(&self, lift: Option<&Path>) -> Conversion {
        if lift.is_some() {
            self.conversion.clone().unwrapped()
        } else {
            self.conversion.clone()
        }
    }

    /// Writes the arm for `target`, which was not refused: its pattern,
    /// the call, and what the arm does with the call's result.
    fn forward(&self, target: &Target, context: &ArmContext) -> Arm {
        let ArmContext {
            kind,
            prefix,
            lift,
            site,
            ..
        } = context;
        let site = *site;
        let route = target.route;
        let conversion = self.conversion_under(*lift);
        let inner = hygienic("__inner", site);
        let receiver = match (kind, self.receiver) {
            // The route is followed through the binding's reference, as for
            // a qualified call, so that `deref` calls the method on what the
            // payload points at and never on the pointer, whose own method
            // of the same name would otherwise win; method-call syntax then
            // borrows the target as the method's receiver asks.
            (ArmKind::MethodCall { .. }, ReceiverKind::Value(access)) => {
                route.reach_for_postfix(&inner, access.borrow().is_some(), site)
            }
            (ArmKind::Place { .. }, _) => quote_spanned!(*site => #inner),
            _ => receiver_expression(self.receiver, target, &inner, site),
        };
        let arguments = self.call_arguments(target, site);
        // The angle brackets are the arm's own tokens: written where the
        // trait's method is, they would stretch the call's span from there
        // to the subject, across every attribute between.
        let turbofish = self
            .turbofish
            .as_ref()
            .map(|parameters| quote_spanned!(*site => ::<#(#parameters),*>));
        let mut value = match kind {
            ArmKind::QualifiedCall { trait_path, method } => {
                // A field's or a method's type cannot be named; the call is
                // then qualified by the trait alone, which still keeps an
                // inherent method of the same name from taking over.
                let qualified = match target.target_type() {
                    Some(target_type) => quote_spanned! { *site => <#target_type as #trait_path> },
                    None => quote_spanned! { *site => <_ as #trait_path> },
                };
                quote_spanned! { *site =>
                    #qualified::#method #turbofish (#receiver #(, #arguments)*)
                }
            }
            ArmKind::MethodCall { name } => {
                quote_spanned! { *site => #receiver.#name #turbofish (#(#arguments),*) }
            }
            // The arm is the target itself, or a field of it: a place,
            // borrowed as the declared return type asks.
            ArmKind::Place { field, borrow } => {
                // The route starts at the binding, which is a reference to
                // the payload only where the pattern binds one: `self` and
                // `self: Box<Self>` are matched by value, and the route
                // starts at the payload itself.
                let through_reference = self.binds_references();
                let place = match field {
                    Some(field) => {
                        let reached = route.reach_for_postfix(&inner, through_reference, site);
                        quote_spanned! { *site => #reached.#field }
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
                        quote_spanned! { *site => *#place }
                    } else {
                        place
                    }
                } else {
                    match borrow {
                        PlaceBorrow::Value | PlaceBorrow::Unread => place,
                        PlaceBorrow::Reference(Borrow::Shared) => {
                            quote_spanned! { *site => &#place }
                        }
                        PlaceBorrow::Reference(Borrow::Exclusive) => {
                            quote_spanned! { *site => &mut #place }
                        }
                    }
                }
            }
        };
        if self.unsafety {
            value = quote_spanned! { *site => unsafe { #value } };
        }
        if self.awaited {
            value = quote_spanned! { *site => #value.await };
        }
        value = convert(value, &conversion, target, prefix, site);
        if let Some(lift) = lift {
            value = quote_spanned! { *site => #lift(#value) };
        }
        Arm {
            attributes: target.arm_attributes(site),
            pattern: self.pattern(target, prefix, &inner, site),
            value,
            site,
        }
    }

    /// Returns the arm that says `error` about `target`: the variant
    /// whatever it holds, with the sentence as its result, under the
    /// variant's own attributes.
    fn sentence_arm(&self, target: &Target, context: &ArmContext, error: Error) -> Arm {
        let site = context.site;
        let sentence = error.to_compile_error();
        self.beside_arguments(Arm {
            attributes: target.arm_attributes(site),
            pattern: target.whole(&context.prefix, site),
            value: quote_spanned!(*site => { #sentence }),
            site,
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
            attributes,
            pattern,
            value,
            site,
        } = arm;
        Arm {
            attributes,
            pattern: quote_spanned!(*site => (#pattern, ..)),
            value,
            site,
        }
    }

    /// Returns the result of an arm that only keeps the `match` exhaustive
    /// beside a variant's own error. It reads the arguments first: in a
    /// build where no other arm is left nothing else uses them, and the
    /// lint about them would stand beside that error. The arm is under its
    /// variant's `cfg`, so a build without the variant has none of this.
    pub(crate) fn standing_in(&self, site: Site) -> TokenStream2 {
        let arguments = &self.arguments;
        if arguments.is_empty() {
            return quote_spanned!(*site => ::core::unreachable!());
        }
        quote_spanned!(*site => {
            let _ = (#(&#arguments,)*);
            ::core::unreachable!()
        })
    }

    /// Returns the expression the `match` looks at: the receiver, with the
    /// same-variant arguments beside it in a tuple.
    pub(crate) fn scrutinee(&self, site: Site) -> TokenStream2 {
        let receiver = receiver_scrutinee(self.receiver, &self.self_token, site);
        if self.by_place {
            return quote_spanned! { *site => *#receiver };
        }
        if self.same_variant.is_empty() {
            return receiver;
        }
        let arguments = self
            .same_variant
            .iter()
            .map(|(index, _)| &self.arguments[*index]);
        quote_spanned! { *site => (#receiver, #(#arguments),*) }
    }

    fn same_binding(index: usize, site: Site) -> Ident {
        hygienic(&format!("__same{index}"), site)
    }

    /// Returns the pattern binding `target`'s payload to `binding`, with the
    /// same-variant arguments' payloads bound beside it. Where the `match`
    /// looks at the subject itself, the binding borrows the payload as the
    /// receiver borrows the subject, so it is the reference it is anywhere
    /// else.
    fn pattern(&self, target: &Target, prefix: &Path, binding: &Ident, site: Site) -> TokenStream2 {
        if self.by_place {
            let borrowed = match self.receiver.borrow() {
                Some(Borrow::Exclusive) => quote_spanned! { *site => ref mut #binding },
                _ => quote_spanned! { *site => ref #binding },
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
        quote_spanned! { *site => (#own, #(#same),*) }
    }

    /// Returns the arguments handed to `target`'s method: each binding as it
    /// is, or the payload of a same-variant argument.
    fn call_arguments(&self, target: &Target, site: Site) -> Vec<TokenStream2> {
        self.arguments
            .iter()
            .enumerate()
            .map(
                |(index, ident)| match self.same_variant.iter().find(|(same, _)| *same == index) {
                    Some((_, access)) => payload_expression(
                        target.route,
                        *access,
                        &Self::same_binding(index, site),
                        site,
                    ),
                    None => quote_spanned!(*site => #ident),
                },
            )
            .collect()
    }

    /// Returns the arm for every pair of different variants, with `value`
    /// as its result. A same-variant argument the scrutinee moved, an owned
    /// or `&mut Self`, is bound again under its own name where `value`
    /// names it, so that a `mismatch` expression may use it; one the
    /// expression does not name is left in the scrutinee, as a hand leaves
    /// it, and no unused binding is written.
    pub(crate) fn fallback(&self, value: TokenStream2, site: Site) -> Arm {
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
                quote_spanned!(*site => _)
            }
        });
        let rebound = self
            .same_variant
            .iter()
            .filter(|same| rebinds(same))
            .map(|(index, _)| {
                let name = &self.arguments[*index];
                let binding = Self::same_binding(*index, site);
                quote_spanned!(*site => let #name = #binding;)
            });
        Arm {
            attributes: vec![],
            pattern: quote_spanned!(*site => (_, #(#positions),*)),
            value: quote_spanned!(*site => { #(#rebound)* #value }),
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

/// Applies `conversion` to `value`, the payload's result, as a hand writes
/// it: the variant's constructor around a `Self`, `Into::into` around an
/// overridden associated type, each as the path a `.map(..)` or a
/// `.map_err(..)` takes, and a tuple taken apart and built again around
/// its converted elements. The target was judged to be rebuilt as the
/// conversion asks.
fn convert(
    value: TokenStream2,
    conversion: &Conversion,
    target: &Target,
    prefix: &Path,
    site: Site,
) -> TokenStream2 {
    // The path a `.map(..)` takes for a conversion of one value.
    let path = |conversion: &Conversion| -> Option<TokenStream2> {
        match conversion {
            Conversion::Keep => None,
            Conversion::Rewrap => Some(
                target
                    .constructor(prefix, site)
                    .expect("the target was judged to be built by a path"),
            ),
            Conversion::Into => Some(quote_spanned! { *site => ::core::convert::Into::into }),
            _ => unreachable!("a conversion under a wrapper is a path"),
        }
    };
    match conversion {
        Conversion::Keep => value,
        Conversion::Rewrap => target
            .rebuild(prefix, value, site)
            .expect("the target was judged to be rebuilt"),
        Conversion::Into => quote_spanned! { *site => ::core::convert::Into::into(#value) },
        Conversion::Map(inner) => match path(inner) {
            Some(path) => quote_spanned! { *site => #value.map(#path) },
            None => value,
        },
        Conversion::MapErr(inner, error) => {
            let mut value = match path(inner) {
                Some(path) => quote_spanned! { *site => #value.map(#path) },
                None => value,
            };
            if let Some(path) = path(error) {
                value = quote_spanned! { *site => #value.map_err(#path) };
            }
            value
        }
        Conversion::Tuple(items) => {
            let bindings: Vec<Ident> = (0..items.len())
                .map(|index| hygienic(&format!("__element{index}"), site))
                .collect();
            let converted = items.iter().zip(&bindings).map(|(item, binding)| {
                let binding = quote_spanned!(*site => #binding);
                convert(binding, item, target, prefix, site)
            });
            quote_spanned! { *site => {
                let (#(#bindings,)*) = #value;
                (#(#converted,)*)
            } }
        }
    }
}

/// Returns the expression the generated `match` looks at for a receiver:
/// the value itself, the enum moved out of its box, or the pinned reference
/// unpinned.
pub(crate) fn receiver_scrutinee(
    kind: ReceiverKind,
    self_token: &Token![self],
    site: Site,
) -> TokenStream2 {
    match kind {
        ReceiverKind::Value(_) => quote_spanned!(*site => #self_token),
        ReceiverKind::Boxed => quote_spanned!(*site => *#self_token),
        ReceiverKind::Pinned(Borrow::Exclusive) => {
            quote_spanned!(*site => ::core::pin::Pin::get_mut(#self_token))
        }
        ReceiverKind::Pinned(Borrow::Shared) => {
            quote_spanned!(*site => ::core::pin::Pin::get_ref(#self_token))
        }
    }
}

/// Returns the expression handing the payload bound to `binding` to the
/// method as its receiver: the payload reached as the receiver borrows, the
/// payload's own box, which the target was judged to hold, or the payload
/// pinned again.
fn receiver_expression(
    kind: ReceiverKind,
    target: &Target,
    binding: &Ident,
    site: Site,
) -> TokenStream2 {
    match kind {
        ReceiverKind::Value(access) => payload_expression(target.route, access, binding, site),
        ReceiverKind::Boxed => quote_spanned!(*site => #binding),
        ReceiverKind::Pinned(borrow) => {
            let payload = payload_expression(target.route, borrow.into(), binding, site);
            quote_spanned!(*site => ::core::pin::Pin::new(#payload))
        }
    }
}

/// Returns the expression that hands `binding`, bound to a variant's
/// payload, to the trait method: the payload itself, or the target the
/// route reaches, borrowed as the receiver is. A route ending in a method
/// call yields a value, which is handed over as it is.
fn payload_expression(route: &Route, access: Access, binding: &Ident, site: Site) -> TokenStream2 {
    if route.is_direct() {
        return quote_spanned!(*site => #binding);
    }
    let reached = route.reach(binding, access.borrow().is_some(), site);
    if route.ends_in_a_method() {
        return reached;
    }
    match access {
        Access::Shared => quote_spanned!(*site => &#reached),
        Access::Exclusive => quote_spanned!(*site => &mut #reached),
        Access::Owned => reached,
    }
}
