//! The forwarding engine: how one method of a subject becomes a `match`
//! with one arm per target.
//!
//! A trait method and an inherent method differ in what each arm calls, in
//! which targets take part and in what the remaining arms do, and agree on
//! the rest: how the receiver is reached, how the arguments are bound, what
//! an arm does with the payload's result, and how the `match` becomes a
//! body. What differs is the method's [`Origin`], which `implement.rs` and
//! `block.rs` name; everything else follows from it in [`forward`]: a
//! [`MethodPlan`] read off the signature, an arm per target, written in
//! `arm.rs`, and the [`Forwarding`] that renders them.
//!
//! Tokens the engine writes carry a `Site`, the subject's or the method's
//! own span: a proc macro's call site, inside the carrier's expansion, is
//! the trait's attribute, which may be in another crate. No module that
//! writes generated code uses a bare `quote!`; a unit test of the crate
//! reads the sources and says so.
use proc_macro2::{Delimiter, Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse_quote_spanned, Block, Error, Expr, FnArg, Ident, Pat, PatIdent, Path, Result, Signature,
    Token,
};

use crate::arm::{hygienic, Arm, ArmContext, ArmKind, Declared};
use crate::options::{ForwardingOptions, TraitOptions};
use crate::refusal::WayOut;
use crate::selection::{check_body, select_targets};
use crate::shape::{
    argument_kind, classify_receiver, classify_return, is_self, place_borrow, turbofish, Access,
    ArgumentKind, Conversion, FutureShape, ReceiverKind, ReturnSite,
};
use crate::site::Site;
use crate::subject::{Entries, Subject};

/// Where a method to forward comes from, with what each of the two paths
/// brings: a trait's, or an inherent block's.
// One per method, and short-lived; the size difference does not matter here.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Origin<'a> {
    /// A method of a trait. Without a mark it is forwarded to every
    /// variant; a partial impl block's mark names the variants and gives
    /// the body for the rest.
    Trait {
        method: TraitMethod<'a>,
        mark: Option<Mark<'a>>,
    },
    /// A method of an inherent block, which always names its variants.
    Inherent { mark: Mark<'a> },
}

impl<'a> Origin<'a> {
    fn mark(&self) -> Option<Mark<'a>> {
        match self {
            Origin::Trait { mark, .. } => *mark,
            Origin::Inherent { mark } => Some(*mark),
        }
    }
}

/// What a trait brings to one of its methods.
pub(crate) struct TraitMethod<'a> {
    /// The trait as the impl names it, which every arm's call is
    /// qualified by.
    pub(crate) trait_path: &'a Path,
    /// The subject's real path, which patterns and values are written on:
    /// its own name, or the path a restatement names.
    pub(crate) subject_path: &'a Path,
    /// The method's signature as the trait declares it, which a sentence
    /// about the method points at.
    pub(crate) declared: &'a Signature,
    /// Whether the trait gives the method a default body, which a method
    /// that cannot be forwarded can be left to.
    pub(crate) has_default: bool,
    /// The partial impl block a sentence points at, spelled out.
    pub(crate) block_header: &'a str,
    /// The trait's options: its overrides decide what a `Self::X` in the
    /// return type converts to.
    pub(crate) options: &'a TraitOptions,
    /// The result for two different variants: the enum's `mismatch` over
    /// the trait method's.
    pub(crate) mismatch: Option<Expr>,
    /// Where the names a `mismatch` on the enum uses resolve.
    pub(crate) resolution: Option<Span>,
    /// Whether the impl is the subject's attribute's,
    /// `#[automatically_derived]`, rather than a partial impl block's,
    /// which is the user's.
    pub(crate) derived: bool,
}

/// A method's `#[dispatch(..)]`: the variants it names, with how each is
/// forwarded to, and the body for the variants it does not name.
#[derive(Clone, Copy)]
pub(crate) struct Mark<'a> {
    pub(crate) options: &'a ForwardingOptions,
    pub(crate) body: Option<&'a Block>,
}

/// One method to forward.
pub(crate) struct Forward<'a> {
    /// The signature the generated method has.
    pub(crate) signature: Signature,
    pub(crate) origin: Origin<'a>,
    pub(crate) entries: &'a Entries<'a>,
    pub(crate) subject: &'a Subject,
    /// The span the arms and the attributes carry.
    pub(crate) site: Site,
    /// The span the body's braces carry: the method's own, so that the
    /// item rustc points at as "defined here" is the method.
    pub(crate) body_site: Site,
}

/// A forwarded method, for its path to put its attributes around.
pub(crate) struct Generated {
    /// The signature, with its parameters bound as the body names them.
    pub(crate) signature: Signature,
    /// The lint allowances a hand writes on the method.
    pub(crate) allowances: TokenStream2,
    pub(crate) body: TokenStream2,
}

/// Forwards one method, which has a receiver: reads its signature, selects
/// its targets, writes an arm for each and the arm for the rest, and
/// renders the `match`.
pub(crate) fn forward(request: Forward) -> Result<Generated> {
    let Forward {
        mut signature,
        origin,
        entries,
        subject,
        site,
        body_site,
    } = request;
    let method = signature.ident.clone();
    let mark = origin.mark();
    // Under `*` an enum's method leaves no variant to a body for the rest,
    // so a body there holds the arms the method writes itself.
    let own_arms = mark.is_some_and(|mark| mark.options.all && mark.body.is_some())
        && !subject.has_one_target();
    let way_out = WayOut {
        on_trait: matches!(origin, Origin::Trait { .. }),
        has_body: mark.is_some_and(|mark| mark.body.is_some()),
        own_arms,
        words: subject.words(),
    };
    let (kind, prefix, lift, declared) = match &origin {
        Origin::Trait {
            method: of_trait, ..
        } => (
            ArmKind::QualifiedCall {
                trait_path: of_trait.trait_path,
                method: &method,
            },
            of_trait.subject_path.clone(),
            None,
            Some(Declared {
                signature: of_trait.declared,
                block_header: of_trait.block_header,
            }),
        ),
        Origin::Inherent { mark } => {
            let options = mark.options;
            // `field` and `value` make the arm a place instead of a call,
            // borrowed as the declared return type asks: `&inner.f` for
            // `-> &T`, `inner.f` for a value, and under `with = Some` or
            // `with = Ok` as what the constructor is handed has to be,
            // `Some(&inner.f)` for `-> Option<&T>`.
            let kind = if options.value || options.field.is_some() {
                ArmKind::Place {
                    field: options.field.as_ref(),
                    borrow: place_borrow(&signature.output, options.with.as_ref()),
                }
            } else {
                ArmKind::MethodCall {
                    name: options.call.as_ref().unwrap_or(&method),
                }
            };
            (
                kind,
                parse_quote_spanned!(*site => Self),
                options.with.as_ref(),
                None,
            )
        }
    };
    let context = ArmContext {
        kind,
        method: &method,
        subject: subject.ident(),
        prefix,
        lift,
        way_out,
        declared,
        site,
    };
    let (names, self_arguments, options, default, resolution) = match &origin {
        // A block method's parameter names are the user's; the trait's are
        // kept where they are plain identifiers and generated otherwise.
        Origin::Trait {
            method: of_trait,
            mark,
        } => (
            match mark {
                Some(_) => ArgumentNames::Written,
                None => ArgumentNames::WrittenOrGenerated,
            },
            SelfArguments::Dispatch,
            Some(of_trait.options),
            of_trait
                .has_default
                .then_some((&method, of_trait.block_header)),
            of_trait.resolution,
        ),
        // In an inherent method `Self` is the subject. For an enum the
        // argument is handed over as it is: a payload method taking the
        // enum is a legitimate target, and matching it on the same variant
        // would need a mismatch the method cannot declare. Two values of a
        // struct always hold the same field, so there the argument is
        // unwrapped to the field, as trait dispatch unwraps it.
        Origin::Inherent { .. } => (
            ArgumentNames::Written,
            if subject.has_one_target() {
                SelfArguments::Dispatch
            } else {
                SelfArguments::PassThrough
            },
            None,
            None,
            None,
        ),
    };
    let plan = MethodPlan::read(
        &mut signature,
        names,
        self_arguments,
        options,
        &ReturnSite {
            subject: subject.ident(),
            one_target: subject.has_one_target(),
            way_out,
            default,
        },
        resolution,
        entries,
    )?;
    // A marked method selects its variants and gives the body for the
    // rest, or under `*` the arms it writes itself; the trait's attribute
    // forwards to every variant.
    let selection = match mark {
        Some(mark) => {
            let selection = select_targets(
                mark.options,
                mark.body,
                plan.matches_self_alone(),
                entries,
                &context.prefix,
                &method,
                subject,
            )?;
            check_body(&selection, mark.body, &signature, subject.words())?;
            Some(selection)
        }
        None => None,
    };
    let own = selection
        .as_ref()
        .and_then(|selection| selection.own.as_ref());
    if let (Some(_), Some(body)) = (own, mark.and_then(|mark| mark.body)) {
        plan.takes_own_arms(&method, subject.ident(), body)?;
    }
    let mut arms = vec![];
    match &selection {
        Some(selection) => {
            for target in &selection.targets {
                let mut arm = plan.arm(target, &context);
                arm.attributes
                    .extend(selection.forwarded_where(target.ident(), site));
                arms.push(arm);
            }
        }
        None => {
            for entry in entries.iter() {
                match entry.target() {
                    Some(target) => arms.push(plan.arm(&target, &context)),
                    // The variant is reported elsewhere, under its `cfg`;
                    // this arm keeps the `match` exhaustive while that
                    // error stands, and reads the arguments, which the
                    // arm for two different variants does not: in a build
                    // that leaves no forwarding arm nothing else would.
                    None => arms.push(plan.beside_arguments(Arm::unforwardable(
                        &context.prefix,
                        entry,
                        plan.standing_in(site),
                        site,
                    ))),
                }
            }
        }
    }
    // A body that holds the method's own arms is no arm for the rest.
    let rest = mark.and_then(|mark| mark.body).filter(|_| own.is_none());
    let Fallback {
        arm: fallback,
        allow_unreachable,
    } = plan.fallback_for(entries, subject, &origin, rest, &context)?;
    let written = own.map(|own| own.tokens(site)).unwrap_or_default();
    if let Some(selection) = selection {
        arms.extend(
            selection
                .unforwardable_arms
                .into_iter()
                .map(|arm| plan.beside_arguments(arm)),
        );
    }
    let allow_unreachable =
        allow_unreachable.then(|| quote_spanned! { *site => #[allow(unreachable_patterns)] });
    // A body for the rest uses the arguments, or is the user's to quiet;
    // the impl the enum's attribute writes is automatically derived.
    let derived = matches!(&origin, Origin::Trait { method, .. } if method.derived);
    let allow_unused = match derived || way_out.has_body {
        true => None,
        false => plan.allow_unused_when_empty(site),
    };
    let body = Forwarding {
        scrutinee: plan.scrutinee(site),
        arms,
        written,
        fallback,
        future: plan.future.clone(),
    }
    .body(body_site);
    Ok(Generated {
        signature,
        allowances: quote_spanned! { *site => #allow_unreachable #allow_unused },
        body,
    })
}

/// How the arguments after the receiver are bound in the generated
/// signature.
#[derive(Clone, Copy)]
enum ArgumentNames {
    /// The identifiers written, so that the generated signature reads like
    /// the trait's; a pattern that is not a plain identifier, which a trait
    /// method with a default body may have, gets a fresh hygienic
    /// `__argument<n>` instead.
    WrittenOrGenerated,
    /// The identifiers written, and an error for any other pattern: an
    /// inherent method's parameters are the user's to rename.
    Written,
}

/// What an argument typed `Self`, `&Self` or `&mut Self` means.
#[derive(Clone, Copy)]
enum SelfArguments {
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

/// What is known about a method before any target is looked at: what its
/// signature says, and what the entries its `match` is over say.
pub(crate) struct MethodPlan {
    pub(crate) receiver: ReceiverKind,
    /// `self` resolves like a local variable, so the generated `match` uses
    /// the receiver's own token to share its hygiene.
    pub(crate) self_token: Token![self],
    /// The binding of every argument after the receiver, in order.
    pub(crate) arguments: Vec<Ident>,
    /// The arguments matched on the same variant as the receiver, by index.
    pub(crate) same_variant: Vec<(usize, Access)>,
    /// The generic arguments a call spells out, when inference could not.
    pub(crate) turbofish: Option<Vec<Ident>>,
    /// What an arm does with the payload's result.
    pub(crate) conversion: Conversion,
    /// The future the method returns, when the `match` sits in one `async
    /// move` block and every arm is awaited inside it.
    pub(crate) future: Option<FutureShape>,
    pub(crate) awaited: bool,
    pub(crate) unsafety: bool,
    /// Whether the method's `where` clause requires `Self: Sized`, which a
    /// trait object payload cannot satisfy.
    pub(crate) requires_sized: bool,
    /// Whether the `match` looks at the subject itself where the receiver
    /// is a reference, `match *self` with `ref` bindings.
    pub(crate) by_place: bool,
    /// Whether every entry the `match` is over is under `#[cfg]`, so that
    /// some build leaves no arm to use the arguments.
    may_be_empty: bool,
}

impl MethodPlan {
    /// Reads `signature`, which must have a receiver, and binds its
    /// arguments as `names` says. `options` carries a trait's associated
    /// type overrides; an inherent method has none. `site` is what the
    /// return type's classification depends on besides the type, and
    /// `entries` are what the `match` is over.
    ///
    /// An enum whose every variant is under `#[cfg]` holds no variant at
    /// all in some build, and a reference to it still counts as inhabited
    /// there, so `match self {}` is not exhaustive while `match *self {}`
    /// is. A hand writes the second over such an enum, `match *self {
    /// Variant(ref inner) => .. }`, and so does the engine, for a reference
    /// receiver. A `Self` argument is matched beside the receiver in a
    /// tuple, which has the fallback arm in every build and stays as it is.
    fn read(
        signature: &mut Signature,
        names: ArgumentNames,
        self_arguments: SelfArguments,
        options: Option<&TraitOptions>,
        site: &ReturnSite,
        resolution: Option<Span>,
        entries: &Entries,
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
                    let ident = hygienic(&format!("__argument{index}"), Site::at(pattern.span()));
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
        let (conversion, future) = classify_return(&signature.output, options, site)?;
        let awaited = signature.asyncness.is_some() || future.is_some();
        let always_present = entries.iter().any(|entry| !entry.is_conditional());
        Ok(Self {
            by_place: receiver.borrow().is_some() && same_variant.is_empty() && !always_present,
            may_be_empty: !always_present,
            receiver,
            self_token,
            arguments,
            same_variant,
            turbofish: turbofish(signature),
            conversion,
            future,
            awaited,
            unsafety: matches!(signature.safety, syn::Safety::Unsafe(_)),
            requires_sized: requires_sized(signature),
        })
    }

    /// Returns the attribute a hand writes on a method over an enum that
    /// may be empty, where a build that leaves no variant leaves the
    /// arguments unused: `#[allow(unused_variables)]`, for a method that
    /// has arguments. An impl the enum's own attribute writes is
    /// `#[automatically_derived]`, under which rustc does not report it,
    /// and takes none.
    fn allow_unused_when_empty(&self, site: Site) -> Option<TokenStream2> {
        (self.may_be_empty && !self.arguments.is_empty())
            .then(|| quote_spanned! { *site => #[allow(unused_variables)] })
    }

    /// Returns whether a pattern over the receiver binds references: the
    /// receiver is one, plain or pinned, where `self` and `self: Box<Self>`
    /// are matched by value and bind the payload itself.
    pub(crate) fn binds_references(&self) -> bool {
        self.receiver.borrow().is_some()
    }

    /// Returns the first `Self` argument matched beside the receiver that
    /// the method takes as `access` says, for a sentence that names what
    /// the method takes.
    pub(crate) fn taken(&self, access: Access) -> Option<&Ident> {
        self.same_variant
            .iter()
            .find(|(_, taken)| *taken == access)
            .map(|(index, _)| &self.arguments[*index])
    }

    /// Returns whether the `match` is `match self` over the receiver
    /// alone, with default binding modes, which is the `match` an arm of
    /// the method's own is written for. A `Self` argument is matched
    /// beside the receiver in a tuple, and an enum whose every variant is
    /// under `#[cfg]` is matched by place with `ref` bindings.
    pub(crate) fn matches_self_alone(&self) -> bool {
        !self.has_same_variant() && !self.by_place
    }

    /// Checks that the method can take arms of its own, which go into the
    /// `match` as they stand: it has to be the `match` they were written
    /// for, see `matches_self_alone`. No surveyed project writes arms of
    /// its own in a method that is matched otherwise, so each is a
    /// sentence, and the named list with a body writes both.
    fn takes_own_arms(&self, method: &Ident, subject: &Ident, body: &Block) -> Result<()> {
        if self.has_same_variant() {
            return Err(Error::new_spanned(
                body,
                format!("`{method}` takes `Self` arguments, which are matched beside `self`, and an arm of its own is written for `self` alone; name the variants in `#[dispatch(..)]` and give the method a body for the rest"),
            ));
        }
        if self.by_place {
            return Err(Error::new_spanned(
                body,
                format!("every variant of `{subject}` is under `#[cfg]`, so `{method}` is matched as `match *self` with `ref` bindings, which arms written for `match self` are not; name the variants in `#[dispatch(..)]` and give the method a body for the rest"),
            ));
        }
        Ok(())
    }

    /// Returns whether a `Self` argument takes part in the `match`, so that
    /// two different variants need a fallback arm.
    pub(crate) fn has_same_variant(&self) -> bool {
        !self.same_variant.is_empty()
    }

    /// Decides the arm for everything the targets' arms leave. A marked
    /// method's body for the rest, `rest`, is the arm for every variant it
    /// does not name, the mismatch of two different variants included, so
    /// it needs no `mismatch`; under `*` the trait's rule applies. Two
    /// values of a struct always hold the same field, so only an enum
    /// needs the arm for two different variants, and an enum with one
    /// variant that is always compiled in has no second variant to meet: a
    /// hand writes no fallback arm there, and neither does the engine. The
    /// arm stays unreachable in some builds when a variant is under `cfg`,
    /// which is the one case the lint is allowed for.
    fn fallback_for(
        &self,
        entries: &Entries,
        subject: &Subject,
        origin: &Origin,
        rest: Option<&Block>,
        context: &ArmContext,
    ) -> Result<Fallback> {
        let none = Fallback {
            arm: None,
            allow_unreachable: false,
        };
        if let Some(body) = rest {
            return Ok(Fallback {
                arm: Some(Arm::rest(body.to_token_stream(), context.site)),
                allow_unreachable: false,
            });
        }
        let always_present = entries
            .iter()
            .filter(|entry| entry.target().is_some() && !entry.is_conditional())
            .count();
        let conditional = entries
            .iter()
            .any(|entry| entry.is_conditional() || entry.unforwardable().is_some());
        let needs_fallback = self.has_same_variant()
            && subject.can_mismatch()
            && (always_present >= 2 || conditional);
        // An inherent method of an enum hands a `Self` argument over as it
        // is, so only a trait's method has two variants to meet.
        let Origin::Trait { method, mark } = origin else {
            return Ok(none);
        };
        if !needs_fallback {
            return Ok(none);
        }
        let Some(mismatch) = &method.mismatch else {
            let name = context.method;
            // The enum's side of the option is read beside the trait: on
            // the enum's attribute that names it, or on the partial block
            // that is the impl.
            let beside_the_trait = if method.derived {
                "on the enum"
            } else {
                "on this block's attribute"
            };
            // A provided method has a third answer, which looks at both
            // values where an expression cannot: the trait's default,
            // which calls the forwarded methods on each.
            let default = match (method.has_default, method.derived) {
                (false, _) => String::new(),
                (true, true) => format!(", or leave the method to the trait's default with `#[dispatch(default)] fn {name}(..);` in a `#[whichever::dispatch] {}` block", method.block_header),
                (true, false) => format!(", or leave the method to the trait's default with `#[dispatch(default)] fn {name}(..);` in this block"),
            };
            return Err(Error::new_spanned(
                method.declared,
                format!("`{name}` takes `Self` arguments; give the result for two different variants with `#[dispatch(mismatch = <expr>)]` on the trait method or `mismatch({name} = <expr>)` {beside_the_trait}{default}"),
            ));
        };
        Ok(Fallback {
            arm: Some(self.fallback(mismatch.to_token_stream(), context.site)),
            allow_unreachable: mark.is_none() && always_present < 2,
        })
    }
}

/// The arm for everything the targets' arms leave, and whether the lint
/// about an unreachable pattern is allowed on the `match`.
struct Fallback {
    arm: Option<Arm>,
    allow_unreachable: bool,
}

/// One generated method body: a `match` over the subject.
pub(crate) struct Forwarding {
    pub(crate) scrutinee: TokenStream2,
    pub(crate) arms: Vec<Arm>,
    /// The arms the method writes itself under `*`, as written, after the
    /// forwarding arms.
    pub(crate) written: TokenStream2,
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
    pub(crate) fn body(&self, site: Site) -> TokenStream2 {
        let span = *site;
        let Forwarding {
            scrutinee,
            arms,
            written,
            fallback,
            future,
        } = self;
        let mut body = quote_spanned! { span => match #scrutinee { #(#arms)* #written #fallback } };
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

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;
    use crate::refusal::Refusal;
    use crate::words::Words;

    fn site() -> Site {
        Site::at(Span::call_site())
    }

    /// An enum with a target of every kind the refusals tell apart.
    fn shapes() -> Subject {
        parse_quote! {
            enum Any {
                Plain(Circle),
                #[dispatch(deref)]
                Boxed(Box<Circle>),
                #[dispatch(deref)]
                Shared(Rc<Circle>),
                #[dispatch(deref)]
                Dyn(Box<dyn Shape>),
                #[dispatch(via = .0)]
                Routed(Wrapper),
                #[dispatch(via = .get())]
                Got(Holder),
                #[dispatch(via = .get(), via_mut = .get_mut())]
                Lent(Holder),
                Tagged(#[dispatch] Circle, u32),
                Named {
                    #[dispatch]
                    inner: Circle,
                },
            }
        }
    }

    fn way_out() -> WayOut {
        WayOut {
            on_trait: true,
            has_body: false,
            own_arms: false,
            words: Words::Enum,
        }
    }

    /// Reads `signature` over `entries` as a trait's method is read.
    fn plan(signature: &Signature, subject: &Subject, entries: &Entries) -> MethodPlan {
        let mut signature = signature.clone();
        let read = MethodPlan::read(
            &mut signature,
            ArgumentNames::WrittenOrGenerated,
            SelfArguments::Dispatch,
            None,
            &ReturnSite {
                subject: subject.ident(),
                one_target: subject.has_one_target(),
                way_out: way_out(),
                default: None,
            },
            None,
            entries,
        );
        match read {
            Ok(plan) => plan,
            Err(error) => panic!("a plan: {error}"),
        }
    }

    /// The refusal a trait's method `signature` meets at `variant` of
    /// `shapes()`, in words.
    fn refusal(signature: Signature, variant: &str) -> &'static str {
        let subject = shapes();
        let entries = subject.read().unwrap();
        let plan = plan(&signature, &subject, &entries);
        let trait_path: Path = parse_quote!(Shape);
        let method = signature.ident.clone();
        let context = ArmContext {
            kind: ArmKind::QualifiedCall {
                trait_path: &trait_path,
                method: &method,
            },
            method: &method,
            subject: subject.ident(),
            prefix: parse_quote!(Any),
            lift: None,
            way_out: way_out(),
            declared: Some(Declared {
                signature: &signature,
                block_header: "impl Shape for Any",
            }),
            site: site(),
        };
        let target = entries
            .targets()
            .into_iter()
            .find(|target| target.ident() == variant)
            .expect("a variant of `shapes()`");
        match plan.refusal(&target, &context) {
            None => "none",
            Some(Refusal::TraitObject { .. }) => "trait object",
            Some(Refusal::RewrapThroughRoute) => "rewrap through a route",
            Some(Refusal::MethodRouteBorrowedMutably(_)) => "method route borrowed mutably",
            Some(Refusal::SharedPointerBorrowedMutably(_)) => "shared pointer borrowed mutably",
            Some(Refusal::MovedOutOfPointer(_)) => "moved out of a pointer",
            Some(Refusal::NoBoxToHandOver) => "no box to hand over",
            Some(Refusal::NotRebuilt) => "not rebuilt",
        }
    }

    #[test]
    fn a_plain_target_takes_any_method() {
        for signature in [
            parse_quote!(fn area(&self) -> u32),
            parse_quote!(fn grow(&mut self)),
            parse_quote!(fn into_area(self) -> u32),
            parse_quote!(fn grown(&self) -> Self),
            parse_quote!(fn halved(&self) -> Option<Self>),
        ] {
            assert_eq!(refusal(signature, "Plain"), "none");
        }
    }

    #[test]
    fn each_target_refuses_what_it_cannot_give() {
        assert_eq!(
            refusal(
                parse_quote!(fn sized(&self) -> u32 where Self: Sized),
                "Dyn"
            ),
            "trait object"
        );
        assert_eq!(
            refusal(
                parse_quote!(fn sized(&self) -> u32 where Self: Sized),
                "Boxed"
            ),
            "none"
        );
        assert_eq!(
            refusal(parse_quote!(fn grown(&self) -> Self), "Boxed"),
            "rewrap through a route"
        );
        assert_eq!(
            refusal(parse_quote!(fn grown(&self) -> Self), "Routed"),
            "rewrap through a route"
        );
        assert_eq!(
            refusal(parse_quote!(fn grow(&mut self)), "Got"),
            "method route borrowed mutably"
        );
        assert_eq!(
            refusal(parse_quote!(fn grow(&mut self)), "Routed"),
            "none",
            "a field is a place"
        );
        assert_eq!(
            refusal(parse_quote!(fn grow(&mut self)), "Shared"),
            "shared pointer borrowed mutably"
        );
        assert_eq!(refusal(parse_quote!(fn grow(&mut self)), "Boxed"), "none");
        assert_eq!(
            refusal(parse_quote!(fn into_area(self) -> u32), "Shared"),
            "moved out of a pointer"
        );
        assert_eq!(
            refusal(parse_quote!(fn into_area(self) -> u32), "Boxed"),
            "none"
        );
        assert_eq!(
            refusal(parse_quote!(fn boxed(self: Box<Self>) -> u32), "Plain"),
            "no box to hand over"
        );
        assert_eq!(
            refusal(parse_quote!(fn boxed(self: Box<Self>) -> u32), "Boxed"),
            "none"
        );
        assert_eq!(
            refusal(parse_quote!(fn grown(&self) -> Self), "Tagged"),
            "not rebuilt"
        );
        assert_eq!(
            refusal(parse_quote!(fn grown(&self) -> Self), "Named"),
            "none"
        );
        assert_eq!(
            refusal(parse_quote!(fn halved(&self) -> Option<Self>), "Named"),
            "not rebuilt",
            "a `Self` under an `Option` is rebuilt by a path, and braces are none"
        );
    }

    #[test]
    fn a_mutable_borrow_through_a_route_of_its_own_is_forwarded() {
        for signature in [
            parse_quote!(fn grow(&mut self)),
            parse_quote!(fn grow(self: Pin<&mut Self>)),
            parse_quote!(fn absorb(&mut self, other: &Self)),
            parse_quote!(fn drain_into(&self, other: &mut Self)),
            parse_quote!(fn area(&self) -> u32),
        ] {
            assert_eq!(refusal(signature, "Lent"), "none");
        }
        for signature in [
            parse_quote!(fn grow(self: Pin<&mut Self>)),
            parse_quote!(fn drain_into(&self, other: &mut Self)),
        ] {
            assert_eq!(
                refusal(signature, "Got"),
                "method route borrowed mutably",
                "no route is named for the mutable borrow"
            );
        }
        assert_eq!(
            refusal(parse_quote!(fn grown(&mut self) -> Self), "Lent"),
            "rewrap through a route"
        );
    }

    #[test]
    fn the_first_reason_that_holds_is_the_one_given() {
        // A trait object is named before the route it is reached by.
        assert_eq!(
            refusal(
                parse_quote!(fn grown(&self) -> Self where Self: Sized),
                "Dyn"
            ),
            "trait object"
        );
        // A route that cannot rewrap is named before the method call that
        // ends it.
        assert_eq!(
            refusal(parse_quote!(fn grown(&mut self) -> Self), "Got"),
            "rewrap through a route"
        );
        // A shared pointer lent mutably is named before the box it is not.
        assert_eq!(
            refusal(
                parse_quote!(fn boxed(self: Box<Self>, other: &mut Self) -> u32),
                "Shared"
            ),
            "shared pointer borrowed mutably"
        );
        // A value moved out of a shared pointer is named before the box it
        // is not.
        assert_eq!(
            refusal(
                parse_quote!(fn boxed(self: Box<Self>, other: Self) -> u32),
                "Shared"
            ),
            "moved out of a pointer"
        );
        // A missing box is named before the variant it cannot rebuild.
        assert_eq!(
            refusal(parse_quote!(fn boxed(self: Box<Self>) -> Self), "Tagged"),
            "no box to hand over"
        );
    }

    #[test]
    fn a_reference_receiver_over_an_enum_that_may_be_empty_matches_the_place() {
        let conditional: Subject = parse_quote! {
            enum Any {
                #[cfg(unix)]
                Unix(Circle),
                #[cfg(not(unix))]
                Other(Circle),
            }
        };
        let entries = conditional.read().unwrap();
        let read = |signature: Signature| plan(&signature, &conditional, &entries);
        let by_reference = read(parse_quote!(fn scaled(&self, by: u32) -> u32));
        assert!(by_reference.by_place && by_reference.may_be_empty);
        assert!(
            by_reference.allow_unused_when_empty(site()).is_some(),
            "a build without a variant leaves `by` unused"
        );
        assert!(read(parse_quote!(fn area(&mut self) -> u32)).by_place);
        assert!(
            !read(parse_quote!(fn area(self) -> u32)).by_place,
            "a value is matched as it is"
        );
        assert!(
            !read(parse_quote!(fn same(&self, other: &Self) -> bool)).by_place,
            "a `Self` argument is matched beside the receiver in a tuple"
        );
        assert!(read(parse_quote!(fn area(&self) -> u32))
            .allow_unused_when_empty(site())
            .is_none());
        let subject = shapes();
        let entries = subject.read().unwrap();
        let always = plan(
            &parse_quote!(fn scaled(&self, by: u32) -> u32),
            &subject,
            &entries,
        );
        assert!(!always.by_place && !always.may_be_empty);
        assert!(always.allow_unused_when_empty(site()).is_none());
    }

    /// The fallback arm a trait method `signature` over `subject` gets,
    /// with its mark's body when given, in words: the arm's value and
    /// whether the lint about an unreachable pattern is allowed, or the
    /// error.
    fn fallback(
        subject: Subject,
        signature: Signature,
        mismatch: Option<Expr>,
        body: Option<Block>,
    ) -> std::result::Result<(Option<String>, bool), String> {
        let entries = subject.read().unwrap();
        let plan = plan(&signature, &subject, &entries);
        let trait_path: Path = parse_quote!(Shape);
        let subject_path: Path = parse_quote!(Any);
        let method = signature.ident.clone();
        let options = TraitOptions {
            overrides: vec![],
            mismatch: vec![],
            routes: vec![],
            mutable_routes: vec![],
        };
        // The body is the arm for the variants a named list leaves; under
        // `*` an enum's body holds the method's own arms instead, and
        // `forward` passes no rest.
        let forwarding: ForwardingOptions = syn::parse_str("A").unwrap();
        let mark = body.as_ref().map(|body| Mark {
            options: &forwarding,
            body: Some(body),
        });
        let origin = Origin::Trait {
            method: TraitMethod {
                trait_path: &trait_path,
                subject_path: &subject_path,
                declared: &signature,
                has_default: false,
                block_header: "impl Shape for Any",
                options: &options,
                mismatch,
                resolution: None,
                derived: true,
            },
            mark,
        };
        let context = ArmContext {
            kind: ArmKind::QualifiedCall {
                trait_path: &trait_path,
                method: &method,
            },
            method: &method,
            subject: subject.ident(),
            prefix: subject_path.clone(),
            lift: None,
            way_out: way_out(),
            declared: None,
            site: site(),
        };
        let rest = origin.mark().and_then(|mark| mark.body);
        match plan.fallback_for(&entries, &subject, &origin, rest, &context) {
            Ok(Fallback {
                arm,
                allow_unreachable,
            }) => Ok((arm.map(|arm| arm.value.to_string()), allow_unreachable)),
            Err(error) => Err(error.to_string()),
        }
    }

    #[test]
    fn the_rest_is_the_marks_body_the_mismatch_or_nothing() {
        let two: Subject = parse_quote!(
            enum Any {
                A(Circle),
                B(Square),
            }
        );
        let same: Signature = parse_quote!(fn same(&self, other: &Self) -> bool);
        let area: Signature = parse_quote!(fn area(&self) -> u32);
        assert_eq!(
            fallback(two.clone(), area.clone(), None, Some(parse_quote!({ 0 }))),
            Ok((Some(quote::quote!({ 0 }).to_string()), false)),
            "a marked method's body answers for the rest"
        );
        assert_eq!(fallback(two.clone(), area, None, None), Ok((None, false)));
        assert_eq!(
            fallback(two.clone(), same.clone(), Some(parse_quote!(false)), None),
            Ok((Some("false".to_owned()), false))
        );
        let error = fallback(two, same.clone(), None, None).unwrap_err();
        assert!(
            error.starts_with(
                "`same` takes `Self` arguments; give the result for two different variants"
            ),
            "{error}"
        );
        let one: Subject = parse_quote!(
            enum Any {
                A(Circle),
            }
        );
        assert_eq!(
            fallback(one, same.clone(), None, None),
            Ok((None, false)),
            "one variant always compiled in has no second to meet"
        );
        let conditional: Subject = parse_quote!(
            enum Any {
                A(Circle),
                #[cfg(unix)]
                B(Square),
            }
        );
        assert_eq!(
            fallback(conditional, same, Some(parse_quote!(false)), None),
            Ok((Some("false".to_owned()), true)),
            "the arm is unreachable in a build without `B`"
        );
    }
}
