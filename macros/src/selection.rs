//! Which targets a method's `#[dispatch(..)]` names, the arms a method
//! under `*` writes itself, and the check on a method's body that goes with
//! the choice: the engine's, shared by the trait path and the inherent
//! path.
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote_spanned, ToTokens};
use syn::{
    parse_quote_spanned, Attribute, Block, Error, Expr, ExprMatch, Ident, Pat, Path, Result,
    Signature, Stmt,
};

use crate::arm::Arm;
use crate::builds::Builds;
use crate::options::ForwardingOptions;
use crate::site::Site;
use crate::subject::{Entries, Subject, Target};
use crate::words::Words;

/// What a method's `#[dispatch(..)]` selects: the targets, with the
/// method's own route when it gives one; the arms for the variants `*`
/// cannot reach; whether every variant is covered, so that a body for the
/// rest would never run; and, under `*`, the arms the method writes itself.
pub(crate) struct Selection<'a> {
    pub(crate) targets: Vec<Target<'a>>,
    pub(crate) unforwardable_arms: Vec<Arm>,
    pub(crate) covers_all: bool,
    pub(crate) own: Option<OwnArms<'a>>,
}

impl Selection<'_> {
    /// Returns the `#[cfg]` the forwarding arm of `variant` takes beside
    /// the variant's own. An arm of the method's own under `#[cfg]`
    /// answers where the `cfg` holds, and the variant is forwarded where
    /// it does not, as a hand writes the two arms.
    pub(crate) fn forwarded_where(&self, variant: &Ident, site: Site) -> Option<Attribute> {
        conditional(&self.own.as_ref()?.leaves(variant), site)
    }
}

/// The arms a method under `*` writes itself: its body, one `match self`
/// whose arms answer for the variants they name, while every other variant
/// is forwarded. `*` names every variant and so leaves none to a body for
/// the rest; the only variants a body can answer for there are the ones it
/// names. What is written is the hand's `match` without the arms that only
/// forward, and what is generated is that `match` in full, with no
/// wildcard, so rustc checks it as it checks the hand's.
pub(crate) struct OwnArms<'a> {
    /// The arms, as written.
    arms: &'a [syn::Arm],
    /// Every variant an arm names, with the builds that arm exists in.
    answered: Vec<(&'a Ident, Builds)>,
}

impl<'a> OwnArms<'a> {
    /// Returns the `match` that `body` is, when it is one `match self` and
    /// nothing else.
    pub(crate) fn written(body: &Block) -> Option<&ExprMatch> {
        let [Stmt::Expr(Expr::Match(written), None)] = body.stmts.as_slice() else {
            return None;
        };
        let Expr::Path(scrutinee) = &*written.expr else {
            return None;
        };
        let plain =
            written.attrs.is_empty() && scrutinee.attrs.is_empty() && scrutinee.qself.is_none();
        (plain && scrutinee.path.is_ident("self")).then_some(written)
    }

    /// Reads `body`, the body of `method` under `*`, into the arms it
    /// holds and the variants they name.
    fn read(body: &'a Block, entries: &Entries, method: &Ident) -> Result<Self> {
        let Some(written) = Self::written(body) else {
            return Err(Error::new_spanned(
                body,
                format!("`{method}` is forwarded to every variant, so its body would never run; make the body one `match self` holding the arms of the variants the enum answers itself, or end the signature with `;`"),
            ));
        };
        let mut answered = vec![];
        for arm in &written.arms {
            let mut named = vec![];
            if !names(&arm.pat, entries, &mut named) {
                return Err(Error::new_spanned(
                    &arm.pat,
                    format!("an arm of `{method}` names the variants it answers, as `Self::Name(..)`, and every other variant is forwarded; this one names none: name the variants it answers, or name the forwarded ones in `#[dispatch(..)]` and make this the method's body"),
                ));
            }
            let builds = Builds::of(&arm.attrs);
            answered.extend(named.into_iter().map(|variant| (variant, builds.clone())));
        }
        Ok(Self {
            arms: &written.arms,
            answered,
        })
    }

    /// Returns the builds in which no arm of the method's own answers for
    /// `variant`, which are the builds the variant is forwarded in.
    fn leaves(&self, variant: &Ident) -> Builds {
        self.answered
            .iter()
            .filter(|(named, _)| *named == variant)
            .fold(Builds::Never, |builds, (_, arm)| builds.or(arm.clone()))
            .complement()
    }

    /// Writes the arms as they stand, each closed by a comma: the last arm
    /// as written is not the last of the `match` it is written into. A
    /// comma the engine adds carries `site`.
    pub(crate) fn tokens(&self, site: Site) -> TokenStream2 {
        let mut tokens = TokenStream2::new();
        for arm in self.arms {
            arm.to_tokens(&mut tokens);
            if arm.comma.is_none() {
                tokens.extend(quote_spanned!(*site => ,));
            }
        }
        tokens
    }
}

/// Collects into `named` the variants `pattern` names by path at its top
/// level, through a guard, an or-pattern, parentheses, a reference or a
/// binding `name @ ..`, and returns whether every alternative names one. A
/// lone identifier is the variant of that name where the enum has one, as
/// rustc reads it, and otherwise a binding of whatever is matched, which
/// like `_` names none.
fn names<'p>(pattern: &'p Pat, entries: &Entries, named: &mut Vec<&'p Ident>) -> bool {
    match pattern {
        Pat::Guard(guarded) => names(&guarded.pat, entries, named),
        Pat::Or(alternatives) => alternatives
            .cases
            .iter()
            .all(|case| names(case, entries, named)),
        Pat::Paren(inner) => names(&inner.pat, entries, named),
        Pat::Reference(inner) => names(&inner.pat, entries, named),
        Pat::Ident(binding) => match &binding.subpat {
            Some((_, inner)) => names(inner, entries, named),
            None => {
                let variant = binding.by_ref.is_none()
                    && binding.mutability.is_none()
                    && entries.iter().any(|entry| entry.ident == binding.ident);
                if variant {
                    named.push(&binding.ident);
                }
                variant
            }
        },
        Pat::Path(variant) if variant.qself.is_none() => last(&variant.path, named),
        Pat::TupleStruct(variant) if variant.qself.is_none() => last(&variant.path, named),
        Pat::Struct(variant) if variant.qself.is_none() => last(&variant.path, named),
        _ => false,
    }
}

/// Pushes the last segment of `path`, the variant a pattern names, and
/// returns whether there was one.
fn last<'p>(path: &'p Path, named: &mut Vec<&'p Ident>) -> bool {
    let Some(segment) = path.segments.last() else {
        return false;
    };
    named.push(&segment.ident);
    true
}

/// Returns the `#[cfg]` that puts an arm in `builds`, or nothing for an
/// arm of every build.
fn conditional(builds: &Builds, site: Site) -> Option<Attribute> {
    let attribute = builds.attribute(*site)?;
    Some(parse_quote_spanned!(*site => #attribute))
}

/// Selects the targets `forwarding` names among `entries`, or every target
/// under `*`. Under `*` an enum's method may write arms of its own in
/// `body`, and a variant such an arm names is the method's to answer, not
/// the forwarding's. A variant that holds no payload cannot be forwarded
/// to, so without an arm of its own the arm for it is an error, which
/// fires only where the variant is compiled in and no arm answers for it.
/// The error names the arm as the way around where the method can take
/// one, which `takes_arms` says, and the named list with a body where it
/// cannot.
pub(crate) fn select_targets<'a>(
    forwarding: &'a ForwardingOptions,
    body: Option<&'a Block>,
    takes_arms: bool,
    entries: &'a Entries,
    prefix: &Path,
    method: &Ident,
    subject: &Subject,
) -> Result<Selection<'a>> {
    let subject_ident = subject.ident();
    let noun = subject.target_noun();
    // A struct has one field and no variant for an arm to name, so a body
    // there is a body for the rest, which `*` leaves nothing to.
    let own = match body {
        Some(body) if forwarding.all && !subject.has_one_target() => {
            Some(OwnArms::read(body, entries, method)?)
        }
        _ => None,
    };
    let left = |variant: &Ident| match &own {
        Some(own) => own.leaves(variant),
        None => Builds::Every,
    };
    let site = Site::at(method.span());
    let mut unforwardable_arms = vec![];
    let selected: Vec<Target> = if forwarding.all {
        for entry in entries.iter() {
            let Some(unforwardable) = entry.unforwardable() else {
                continue;
            };
            let builds = left(&entry.ident);
            if builds.is_never() {
                continue;
            }
            let (ident, reason, rest) = (&entry.ident, &unforwardable.reason, unforwardable.rest);
            let way_around = if takes_arms {
                format!("write its arm in the method's body, `match self {{ Self::{ident}{rest} => .. }}`")
            } else {
                String::from(
                    "name the variants with a payload and give the method a body for the rest",
                )
            };
            let message = format!(
                "`{method}` is forwarded to every variant, but `{ident}` {reason}; {way_around}"
            );
            let mut arm = Arm::unforwardable(
                prefix,
                entry,
                quote_spanned! { method.span() => ::core::compile_error!(#message) },
                site,
            );
            arm.attributes.extend(conditional(&builds, site));
            unforwardable_arms.push(arm);
        }
        // On an enum `*` is every variant no arm of the method's own
        // answers for; on a struct, the field the trait is forwarded to,
        // or in an inherent block the field marked bare or the one field.
        entries
            .star()?
            .into_iter()
            .filter(|target| !left(target.ident()).is_never())
            .collect()
    } else {
        forwarding
            .variants
            .iter()
            .map(|name| {
                let entry = entries.iter().find(|entry| entry.ident == *name);
                if let Some(target) = entry.and_then(|entry| entry.target()) {
                    return Ok(target);
                }
                let message = match entry.and_then(|entry| entry.unforwardable()) {
                    Some(unforwardable) => format!(
                        "`{name}` {}, so nothing can be forwarded to it",
                        unforwardable.reason
                    ),
                    None => match entries.forwarded_elsewhere(name) {
                        // A partial block's method on a struct: the trait
                        // goes to one field, and this is another.
                        Some(kept) => format!("`{name}` is a field of `{subject_ident}`, but this trait is forwarded to `{kept}`, and a trait is forwarded to one field; name the trait on `{name}`, `#[dispatch(Trait)]`, to forward it there"),
                        None => format!("`{subject_ident}` has no {noun} `{name}`"),
                    },
                };
                Err(Error::new(name.span(), message))
            })
            .collect::<Result<_>>()?
    };
    // A struct always holds every field it names, so one field covers it.
    let covers_all = forwarding.all
        || subject.has_one_target()
        || entries
            .iter()
            .all(|entry| entry.target().is_some() && forwarding.variants.contains(&entry.ident));
    // The method's own route, when it gives one, over the variants'.
    let targets = selected
        .into_iter()
        .map(|target| match &forwarding.via {
            Some(route) => target.through(route),
            None => target,
        })
        .collect();
    Ok(Selection {
        targets,
        unforwardable_arms,
        covers_all,
        own,
    })
}

/// Checks that a method has a body for the rest exactly when some variant
/// is left to it. The arms a method under `*` writes itself are no such
/// body.
pub(crate) fn check_body(
    selection: &Selection,
    body: Option<&Block>,
    sig: &Signature,
    words: Words,
) -> Result<()> {
    let method = &sig.ident;
    match (selection.covers_all, body) {
        (true, Some(_)) if selection.own.is_some() => Ok(()),
        (true, Some(body)) => Err(Error::new_spanned(
            body,
            match words {
                Words::Enum => format!("`{method}` is forwarded to every variant, so its body would never run; end the signature with `;` instead"),
                // A struct holds its field in every value, so the one
                // field a method is forwarded to leaves a body nothing.
                Words::Struct => format!("`{method}` is forwarded to the struct's field, so its body would never run; end the signature with `;` instead"),
            },
        )),
        (false, None) => Err(Error::new_spanned(
            sig,
            format!("`{method}` is forwarded to some variants only; give it a body, which is what the other variants return"),
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    fn site() -> Site {
        Site::at(Span::call_site())
    }

    /// An enum with forwarded variants, one of them conditional, and a
    /// variant of every shape that holds no single payload.
    fn codecs() -> Subject {
        parse_quote! {
            enum Codec {
                Avc1(Avc1),
                Hev1(Hev1),
                #[cfg(unix)]
                Vp09(Vp09),
                Unknown(FourCc, Vec<u8>),
                Skip,
                Labelled { label: u32 },
            }
        }
    }

    fn spelled(tokens: impl ToTokens) -> String {
        tokens.to_token_stream().to_string()
    }

    fn message<T>(result: Result<T>) -> String {
        match result {
            Ok(_) => panic!("expected an error"),
            Err(error) => error.to_string(),
        }
    }

    /// The variants `pattern` names among those of `codecs()`, or nothing
    /// when some alternative of it names none.
    fn named(pattern: Pat) -> Option<Vec<String>> {
        let subject = codecs();
        let entries = subject.read().unwrap();
        let mut named = vec![];
        if !names(&pattern, &entries, &mut named) {
            return None;
        }
        Some(named.iter().map(ToString::to_string).collect())
    }

    /// What `select_targets` makes of a method of `codecs()` marked
    /// `forwarding` with `body`, in words.
    struct Selected {
        /// The targets, each with the `#[cfg]` its forwarding arm takes
        /// beside the variant's own.
        targets: Vec<(String, Option<String>)>,
        /// The error arms, each as its attributes and its pattern, which
        /// matches the variant whatever it holds, `Codec::Skip { .. }`.
        refused: Vec<(Vec<String>, String)>,
        /// The values of the error arms.
        values: Vec<String>,
        covers_all: bool,
        own: bool,
    }

    fn select(subject: Subject, forwarding: TokenStream2, body: Option<Block>) -> Result<Selected> {
        let forwarding: ForwardingOptions = syn::parse2(forwarding).unwrap();
        let entries = subject.read().unwrap();
        let prefix: Path = parse_quote!(Codec);
        let method: Ident = parse_quote!(encode);
        let selection = select_targets(
            &forwarding,
            body.as_ref(),
            true,
            &entries,
            &prefix,
            &method,
            &subject,
        )?;
        Ok(Selected {
            targets: selection
                .targets
                .iter()
                .map(|target| {
                    let ident = target.ident();
                    (
                        ident.to_string(),
                        selection.forwarded_where(ident, site()).map(spelled),
                    )
                })
                .collect(),
            refused: selection
                .unforwardable_arms
                .iter()
                .map(|arm| {
                    (
                        arm.attributes.iter().map(spelled).collect(),
                        arm.pattern.to_string(),
                    )
                })
                .collect(),
            values: selection
                .unforwardable_arms
                .iter()
                .map(|arm| arm.value.to_string())
                .collect(),
            covers_all: selection.covers_all,
            own: selection.own.is_some(),
        })
    }

    fn target_names(selected: &Selected) -> Vec<&str> {
        selected
            .targets
            .iter()
            .map(|(name, _)| name.as_str())
            .collect()
    }

    fn refused_patterns(selected: &Selected) -> Vec<String> {
        selected
            .refused
            .iter()
            .map(|(_, pattern)| pattern.clone())
            .collect()
    }

    #[test]
    fn the_arms_of_a_method_are_one_match_self_and_nothing_else() {
        let written: Block = parse_quote!({
            match self {
                Self::Skip => 0,
            }
        });
        let found = OwnArms::written(&written).expect("one `match self`");
        assert_eq!(found.arms.len(), 1);
        for body in [
            parse_quote!({
                let zero = 0;
                match self {
                    Self::Skip => zero,
                }
            }),
            parse_quote!({
                match self {
                    Self::Skip => 0,
                };
            }),
            parse_quote!({
                match *self {
                    Self::Skip => 0,
                }
            }),
            parse_quote!({
                match other {
                    Self::Skip => 0,
                }
            }),
            parse_quote!({
                #[allow(unreachable_patterns)]
                match self {
                    Self::Skip => 0,
                }
            }),
            parse_quote!({
                match self {
                    Self::Skip => 0,
                }
                0
            }),
            parse_quote!({ 0 }),
            parse_quote!({}),
        ] {
            let body: Block = body;
            assert!(
                OwnArms::written(&body).is_none(),
                "not one `match self`: {}",
                spelled(&body)
            );
        }
    }

    #[test]
    fn an_arm_names_the_variants_its_pattern_names_by_path_at_the_top() {
        let one = |name: &str| Some(vec![name.to_owned()]);
        assert_eq!(named(parse_quote!(Self::Skip)), one("Skip"));
        assert_eq!(named(parse_quote!(Codec::Skip)), one("Skip"));
        assert_eq!(named(parse_quote!(Self::Avc1(inner))), one("Avc1"));
        assert_eq!(
            named(parse_quote!(Self::Unknown(kind, data))),
            one("Unknown")
        );
        assert_eq!(
            named(parse_quote!(Self::Labelled { label })),
            one("Labelled")
        );
        assert_eq!(named(parse_quote!(Self::Labelled { .. })), one("Labelled"));
        assert_eq!(
            named(parse_quote!(Self::Avc1(inner) if inner.is_empty())),
            one("Avc1"),
            "a guard names what its pattern names"
        );
        assert_eq!(
            named(parse_quote!(Self::Avc1(..) | Self::Hev1(..))),
            Some(vec!["Avc1".to_owned(), "Hev1".to_owned()])
        );
        assert_eq!(named(parse_quote!((Self::Skip))), one("Skip"));
        assert_eq!(named(parse_quote!(&Self::Skip)), one("Skip"));
        assert_eq!(named(parse_quote!(&mut Self::Skip)), one("Skip"));
        assert_eq!(
            named(parse_quote!(whole @ Self::Unknown(..))),
            one("Unknown")
        );
        assert_eq!(
            named(parse_quote!(Skip)),
            one("Skip"),
            "a lone identifier is the variant of that name"
        );
        assert_eq!(named(parse_quote!(_)), None);
        assert_eq!(
            named(parse_quote!(other)),
            None,
            "a lone identifier the enum has no variant of is a binding"
        );
        assert_eq!(
            named(parse_quote!(ref Skip)),
            None,
            "a `ref` identifier is a binding"
        );
        assert_eq!(named(parse_quote!(0)), None);
        assert_eq!(named(parse_quote!(whole @ _)), None);
        assert_eq!(
            named(parse_quote!(Self::Skip | _)),
            None,
            "an alternative that names none leaves the arm naming none"
        );
    }

    #[test]
    fn a_variant_an_arm_names_under_cfg_is_forwarded_where_the_cfg_does_not_hold() {
        let subject = codecs();
        let entries = subject.read().unwrap();
        let method: Ident = parse_quote!(encode);
        let body: Block = parse_quote!({
            match self {
                Self::Skip => 0,
                #[cfg(feature = "unknown")]
                Self::Unknown(..) => 1,
                #[cfg(feature = "first")]
                Self::Avc1(..) => 2,
                #[cfg(feature = "second")]
                Self::Avc1(..) => 3,
                #[cfg(feature = "labelled")]
                Self::Labelled { .. } => 4,
                Self::Labelled { .. } => 5,
            }
        });
        let own = match OwnArms::read(&body, &entries, &method) {
            Ok(own) => own,
            Err(error) => panic!("arms of the method's own: {error}"),
        };
        let leaves = |name: &str| own.leaves(&Ident::new(name, Span::call_site()));
        assert!(
            leaves("Skip").is_never(),
            "an arm without a `cfg` answers in every build"
        );
        assert!(leaves("Hev1").is_every(), "no arm names it");
        assert_eq!(
            leaves("Unknown").attribute(Span::call_site()).map(spelled),
            Some(spelled(quote!(#[cfg(not(feature = "unknown"))])))
        );
        assert_eq!(
            leaves("Avc1").attribute(Span::call_site()).map(spelled),
            Some(spelled(
                quote!(#[cfg(not(any(feature = "first", feature = "second")))])
            )),
            "two conditional arms answer where either holds"
        );
        assert!(
            leaves("Labelled").is_never(),
            "an arm in every build answers beside a conditional one"
        );
        assert_eq!(
            spelled(own.tokens(site())),
            spelled(quote! {
                Self::Skip => 0,
                #[cfg(feature = "unknown")]
                Self::Unknown(..) => 1,
                #[cfg(feature = "first")]
                Self::Avc1(..) => 2,
                #[cfg(feature = "second")]
                Self::Avc1(..) => 3,
                #[cfg(feature = "labelled")]
                Self::Labelled { .. } => 4,
                Self::Labelled { .. } => 5,
            }),
            "the arms are written as they stand, the last one closed by a comma"
        );
    }

    #[test]
    fn a_block_arm_is_closed_by_a_comma_it_was_written_without() {
        let subject = codecs();
        let entries = subject.read().unwrap();
        let method: Ident = parse_quote!(encode);
        let body: Block = parse_quote!({
            match self {
                Self::Skip => {}
                Self::Unknown(..) => {}
            }
        });
        let Ok(own) = OwnArms::read(&body, &entries, &method) else {
            panic!("arms of the method's own");
        };
        assert_eq!(
            spelled(own.tokens(site())),
            spelled(quote!(Self::Skip => {}, Self::Unknown(..) => {},))
        );
    }

    #[test]
    fn under_star_the_arms_leave_every_variant_they_do_not_name_to_the_forwarding() {
        let selected = select(
            codecs(),
            quote!(*),
            Some(parse_quote!({
                match self {
                    Self::Unknown(kind, data) => 0,
                    Self::Skip | Self::Labelled { .. } => 1,
                    Self::Hev1(..) => 2,
                }
            })),
        )
        .unwrap();
        assert!(selected.own && selected.covers_all);
        assert_eq!(
            target_names(&selected),
            ["Avc1", "Vp09"],
            "a variant that holds a payload is answered by its arm all the same"
        );
        assert!(selected
            .targets
            .iter()
            .all(|(_, forwarded_where)| forwarded_where.is_none()));
        assert!(
            selected.refused.is_empty(),
            "every variant without a payload has its arm"
        );
    }

    #[test]
    fn under_star_a_variant_without_a_payload_or_an_arm_is_an_error_naming_the_arm() {
        let selected = select(
            codecs(),
            quote!(*),
            Some(parse_quote!({
                match self {
                    Self::Skip => 0,
                }
            })),
        )
        .unwrap();
        assert_eq!(target_names(&selected), ["Avc1", "Hev1", "Vp09"]);
        assert_eq!(
            refused_patterns(&selected),
            [
                spelled(quote!(Codec::Unknown { .. })),
                spelled(quote!(Codec::Labelled { .. })),
            ]
        );
        assert!(
            selected.values[0].contains(
                "write its arm in the method's body, `match self { Self::Unknown(..) => .. }`"
            ),
            "{}",
            selected.values[0]
        );
        assert!(
            selected.values[1].contains(
                "write its arm in the method's body, `match self { Self::Labelled { .. } => .. }`"
            ),
            "{}",
            selected.values[1]
        );
        let bodiless = select(codecs(), quote!(*), None).unwrap();
        assert!(!bodiless.own);
        assert_eq!(target_names(&bodiless), ["Avc1", "Hev1", "Vp09"]);
        assert_eq!(
            refused_patterns(&bodiless),
            [
                spelled(quote!(Codec::Unknown { .. })),
                spelled(quote!(Codec::Skip { .. })),
                spelled(quote!(Codec::Labelled { .. })),
            ],
            "without a body every variant without a payload is an error"
        );
        assert!(bodiless
            .refused
            .iter()
            .all(|(attributes, _)| attributes.is_empty()));
        assert!(bodiless.values[1].contains("`match self { Self::Skip => .. }`"));
    }

    #[test]
    fn the_way_around_a_variant_without_a_payload_is_an_arm_only_where_the_method_takes_one() {
        let subject = codecs();
        let entries = subject.read().unwrap();
        let forwarding: ForwardingOptions = parse_quote!(*);
        let method: Ident = parse_quote!(encode);
        let value = |takes_arms: bool| {
            let selection = select_targets(
                &forwarding,
                None,
                takes_arms,
                &entries,
                &parse_quote!(Codec),
                &method,
                &subject,
            )
            .unwrap();
            let skip = selection
                .unforwardable_arms
                .iter()
                .find(|arm| arm.pattern.to_string() == spelled(quote!(Codec::Skip { .. })))
                .expect("the error arm of `Skip`");
            skip.value.to_string()
        };
        let takes = value(true);
        assert!(
            takes.contains(
                "`encode` is forwarded to every variant, but `Skip` holds no payload; write its arm in the method's body, `match self { Self::Skip => .. }`"
            ),
            "{takes}"
        );
        let cannot = value(false);
        assert!(
            cannot.contains(
                "`encode` is forwarded to every variant, but `Skip` holds no payload; name the variants with a payload and give the method a body for the rest"
            ),
            "a `Self` argument or a match by place takes no arms of its own: {cannot}"
        );
    }

    #[test]
    fn under_star_a_conditional_arm_leaves_its_variant_to_the_forwarding_elsewhere() {
        let selected = select(
            codecs(),
            quote!(*),
            Some(parse_quote!({
                match self {
                    #[cfg(feature = "skip")]
                    Self::Skip => 0,
                    #[cfg(feature = "avc")]
                    Self::Avc1(..) => 1,
                    Self::Unknown(..) | Self::Labelled { .. } => 2,
                }
            })),
        )
        .unwrap();
        assert_eq!(
            selected.targets,
            [
                (
                    "Avc1".to_owned(),
                    Some(spelled(quote!(#[cfg(not(feature = "avc"))])))
                ),
                ("Hev1".to_owned(), None),
                ("Vp09".to_owned(), None),
            ]
        );
        assert_eq!(
            selected.refused,
            [(
                vec![spelled(quote!(#[cfg(not(feature = "skip"))]))],
                spelled(quote!(Codec::Skip { .. }))
            )],
            "the error fires only where no arm answers for the variant"
        );
    }

    #[test]
    fn under_star_a_body_that_is_not_one_match_self_or_an_arm_naming_none_is_reported() {
        let error = message(select(codecs(), quote!(*), Some(parse_quote!({ 0 }))));
        assert_eq!(
            error,
            "`encode` is forwarded to every variant, so its body would never run; make the body one `match self` holding the arms of the variants the enum answers itself, or end the signature with `;`"
        );
        let error = message(select(
            codecs(),
            quote!(*),
            Some(parse_quote!({
                match self {
                    Self::Skip => 0,
                    _ => 1,
                }
            })),
        ));
        assert!(
            error.starts_with(
                "an arm of `encode` names the variants it answers, as `Self::Name(..)`, and every other variant is forwarded; this one names none"
            ),
            "{error}"
        );
    }

    #[test]
    fn a_named_list_keeps_its_body_for_the_rest_and_a_struct_has_no_arms() {
        let listed = select(
            codecs(),
            quote!(Avc1, Hev1),
            Some(parse_quote!({
                match self {
                    Self::Skip => 0,
                }
            })),
        )
        .unwrap();
        assert!(!listed.own && !listed.covers_all);
        assert_eq!(target_names(&listed), ["Avc1", "Hev1"]);
        let wrapper: Subject = parse_quote!(
            struct Wrapper(Avc1);
        );
        let wrapped = select(
            wrapper.clone(),
            quote!(*),
            Some(parse_quote!({
                match self {
                    Self(inner) => 0,
                }
            })),
        )
        .unwrap();
        assert!(
            !wrapped.own && wrapped.covers_all,
            "a struct has no variant to name"
        );
        let entries = wrapper.read().unwrap();
        let forwarding: ForwardingOptions = parse_quote!(*);
        let method: Ident = parse_quote!(encode);
        let body: Block = parse_quote!({ 0 });
        let selection = select_targets(
            &forwarding,
            Some(&body),
            true,
            &entries,
            &parse_quote!(Wrapper),
            &method,
            &wrapper,
        )
        .unwrap();
        let signature: Signature = parse_quote!(fn encode(&self) -> u32);
        assert_eq!(
            message(check_body(
                &selection,
                Some(&body),
                &signature,
                wrapper.words()
            )),
            "`encode` is forwarded to the struct's field, so its body would never run; end the signature with `;` instead"
        );
    }

    #[test]
    fn the_arms_of_a_method_under_star_are_no_body_for_the_rest() {
        let subject = codecs();
        let entries = subject.read().unwrap();
        let forwarding: ForwardingOptions = parse_quote!(*);
        let method: Ident = parse_quote!(encode);
        let signature: Signature = parse_quote!(fn encode(&self) -> u32);
        let body: Block = parse_quote!({
            match self {
                Self::Skip | Self::Unknown(..) | Self::Labelled { .. } => 0,
            }
        });
        let selection = select_targets(
            &forwarding,
            Some(&body),
            true,
            &entries,
            &parse_quote!(Codec),
            &method,
            &subject,
        )
        .unwrap();
        assert!(check_body(&selection, Some(&body), &signature, subject.words()).is_ok());
        let every: ForwardingOptions = parse_quote!(Avc1, Hev1);
        let two: Subject = parse_quote!(
            enum Codec {
                Avc1(Avc1),
                Hev1(Hev1),
            }
        );
        let entries = two.read().unwrap();
        let listed = select_targets(
            &every,
            Some(&body),
            true,
            &entries,
            &parse_quote!(Codec),
            &method,
            &two,
        )
        .unwrap();
        assert_eq!(
            message(check_body(&listed, Some(&body), &signature, two.words())),
            "`encode` is forwarded to every variant, so its body would never run; end the signature with `;` instead",
            "a named list that names every variant leaves its body nothing"
        );
    }
}
