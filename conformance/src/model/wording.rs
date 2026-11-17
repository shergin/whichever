//! What a sentence has to say for a cell beside its needle: the way out
//! its catalogue entry names, the method's own way out where it ends in
//! one, and the names it interpolates, computed from the cell. The
//! wording is the snapshots' in `tests/ui`.

use super::argument::ArgumentKind;
use super::limits::{
    Anchor, Expected, Limit, Tail, BODY_NEVER_RUNS_UNDER_STAR,
    STAR_OVER_UNFORWARDABLE_BY_NAMED_LIST,
};
use super::path::{Form as PathForm, Site as PathSite};
use super::receiver::Lends;
use super::target::{Step, TargetDescriptor, TRAIT_ON_FIELD};
use super::{Callee, Receiver, Slot, SlotVariant};
use crate::cell::Cell;
use crate::render::context::storage;

/// What a sentence has to hold: one of the texts, in full. A sentence
/// that each of several variants gets on one anchor names one of them, so
/// its name is any of theirs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment(pub Vec<String>);

impl Fragment {
    /// Returns a fragment of one text.
    pub fn one(text: String) -> Fragment {
        Fragment(vec![text])
    }

    /// Returns whether a message holds one of the texts.
    pub fn is_in(&self, message: &str) -> bool {
        self.0.iter().any(|text| message.contains(text.as_str()))
    }
}

/// Returns the fragments a sentence the model predicts has to hold.
pub fn fragments(cell: &Cell, expected: &Expected) -> Vec<Fragment> {
    let entry = expected.limit.entry();
    let slot = match expected.anchor {
        Anchor::Variant(index) => cell
            .declared_slots()
            .into_iter()
            .find(|slot| slot.index == index),
        Anchor::Method
        | Anchor::Attribute
        | Anchor::Enum
        | Anchor::BlockMethod
        | Anchor::BlockSignature
        | Anchor::BlockAttribute
        | Anchor::OtherAttribute
        | Anchor::BlockHeader
        | Anchor::BlockInvocation
        | Anchor::Arm
        | Anchor::Associated
        | Anchor::Anywhere => None,
    };
    let variant = slot.map(|slot| named(cell, &slot));
    // The way out of a rebuild that would drop the other fields is the
    // whole of the sentence's advice: where the method names its variants
    // and has a body for the rest, it is that body
    // (`self_return_beside_the_other_fields_of_a_variant` holds the other
    // form), and the entry's own way out is not written.
    //
    // Open question: no snapshot holds the sentence on an inherent method;
    // there the entry's way out, a `#[whichever::dispatch] impl` block, is
    // where the method already is, so it is read as ending in the
    // inherent method's own way out, a body.
    //
    // Under `*` with arms of the method's own, it is the variant's arm
    // (`own_arms_over_a_variant_that_cannot_be_rebuilt`).
    let body_for_the_rest = matches!(
        cell.path_descriptor().form,
        PathForm::First | PathForm::Arms
    );
    let inherent = cell.path_descriptor().site == PathSite::Inherent;
    let rebuild_left_to_the_body =
        expected.limit == Limit::SelfReturnOtherFields && (body_for_the_rest || inherent);
    // A method that holds no arms of its own under `*` keeps the named
    // list as its way out, where another writes the variant's arm.
    let star_by_named_list =
        expected.limit == Limit::StarOverUnforwardable && !cell.takes_own_arms();
    // A body under `*` on an enum may hold the arms the enum answers
    // itself, which its sentence offers; a named list that names every
    // variant, and a struct, which has no variant to name, keep the entry's.
    let body_under_star = expected.limit == Limit::BodyNeverRuns
        && cell.path_descriptor().form == PathForm::Arms
        && !cell.s0.descriptor().is_struct();
    let mut fragments = Vec::new();
    if body_under_star {
        fragments.push(Fragment::one(String::from(BODY_NEVER_RUNS_UNDER_STAR)));
    } else if star_by_named_list {
        fragments.push(Fragment::one(String::from(
            STAR_OVER_UNFORWARDABLE_BY_NAMED_LIST,
        )));
    } else if !rebuild_left_to_the_body {
        fragments.push(Fragment::one(String::from(entry.way_out)));
    }
    if entry.tail || rebuild_left_to_the_body {
        if let Some(tail) = tail(cell, variant.as_deref()) {
            fragments.push(Fragment::one(tail));
        }
    }
    fragments.extend(names(cell, expected.limit, slot));
    fragments
}

/// Returns how a sentence names the variant of a slot, in backticks: by
/// its name, or on a struct by the struct's, as the snapshot
/// `struct_self_return_other_fields` names it.
///
/// Open question: no snapshot holds a sentence about a struct's field
/// under any other limit; the struct's name is read as standing for it in
/// every sentence.
fn named(cell: &Cell, slot: &Slot) -> String {
    match cell.s0.descriptor().is_struct() {
        true => String::from("`E`"),
        false => format!("`{}`", slot.name()),
    }
}

/// Returns how the method's own way out ends the sentence: a body for an
/// inherent method that has none (`place_moves_through_a_shared_pointer_
/// under_a_boxed_receiver`), the body it has for a variant a method names,
/// the mark dropped for a sentence on the whole of a method that has a
/// body (`lift_over_a_wrapped_self`), on the inherent path and a partial
/// block's alike, and by hand for a trait's method, a block's bodiless
/// mark included. Under `*` with arms of its own it is the variant's arm,
/// or for a sentence on the whole method the mark dropped and every
/// variant's arm written (`own_arms_over_a_variant_the_method_refuses`,
/// `own_arms_beside_a_self_return_the_lift_cannot_wrap`).
fn tail(cell: &Cell, variant: Option<&str>) -> Option<String> {
    let path = cell.path_descriptor();
    let tail = match (path.form, path.site) {
        (PathForm::First, _) => Tail::LeaveToBody,
        (PathForm::Arms, _) => Tail::ArmInBody,
        (PathForm::Forwarded | PathForm::Default | PathForm::Written, PathSite::Inherent) => {
            Tail::Body
        }
        (
            PathForm::Forwarded | PathForm::Default | PathForm::Written,
            PathSite::Attribute | PathSite::Block,
        ) => Tail::ByHand,
    };
    match (tail, variant) {
        (Tail::LeaveToBody, None) => Some(Tail::DropMark.fragment("")),
        (Tail::LeaveToBody, Some(variant)) => Some(tail.fragment(variant)),
        (Tail::ArmInBody, None) => Some(Tail::DropStar.fragment("")),
        (Tail::ArmInBody, Some(variant)) => Some(tail.fragment(variant)),
        (Tail::ByHand | Tail::Body | Tail::DropMark | Tail::DropStar, _) => Some(tail.fragment("")),
    }
}

/// Returns what the method takes as a sentence spells it, for the
/// sentences that say why a borrow or a move cannot be made: the receiver
/// as the signature has it, or the `Self` argument where it is what
/// borrows or moves.
fn takes(cell: &Cell, limit: Limit) -> String {
    let receiver = cell.recv.descriptor();
    let argument = cell.reached_argument().kind;
    let place = matches!(cell.callee, Callee::Value | Callee::Field);
    match limit {
        Limit::BoxSelfOnSharedPointer => String::from("takes `self: Box<Self>`"),
        Limit::MovesThroughSharedPointer => match cell.recv {
            Receiver::Owned => String::from("takes `self` by value"),
            Receiver::Boxed if place => String::from("takes `self: Box<Self>`"),
            Receiver::Shared
            | Receiver::Exclusive
            | Receiver::Boxed
            | Receiver::PinShared
            | Receiver::PinExclusive
            | Receiver::Rc
            | Receiver::Arc
            | Receiver::PinBoxed => String::from("takes `other` by value"),
        },
        Limit::MutableBorrowThroughSharedPointer | Limit::MethodRouteOnMutSelf => {
            let inherent = cell.path_descriptor().site == PathSite::Inherent;
            if inherent && place && limit == Limit::MutableBorrowThroughSharedPointer {
                return String::from("returns a `&mut`");
            }
            match (receiver.lends, argument) {
                (Lends::Exclusive, _) => format!("takes `{}`", receiver.spelling),
                (Lends::Shared | Lends::Nothing, Some(ArgumentKind::Exclusive)) => {
                    String::from("takes `other: &mut Self`")
                }
                (Lends::Shared | Lends::Nothing, _) => format!("takes `{}`", receiver.spelling),
            }
        }
        Limit::Receiver
        | Limit::UnitVariant
        | Limit::TwoFields
        | Limit::SelfArgumentWithoutMismatch
        | Limit::OpaqueReturn
        | Limit::SelfInReturn
        | Limit::DerefSelfReturn
        | Limit::RouteSelfReturn
        | Limit::SelfReturnOtherFields
        | Limit::BoxSelfPlain
        | Limit::NamedFieldUnderWrapper
        | Limit::NamedVariantUnderWrapper
        | Limit::VariantFieldOption
        | Limit::MacroInBlock
        | Limit::UnsuppliedType
        | Limit::DefaultWithoutDefault
        | Limit::BodyNeverRuns
        | Limit::StarOverUnforwardable
        | Limit::StructConversion
        | Limit::IntoOtherFields
        | Limit::IntoWithoutPayload
        | Limit::RestatedConversions
        | Limit::TypeDisagreement
        | Limit::ConstDisagreement
        | Limit::FromOverlapTypeParameter
        | Limit::TryFromTypeParameter
        | Limit::ArmsDisagree
        | Limit::MismatchForAbsentMethod
        | Limit::SizedOnObject
        | Limit::GenericAssociatedType
        | Limit::ArmNamingNoVariant
        | Limit::ArmsBesideSelfArgument
        | Limit::ArmsMatchedByPlace
        | Limit::FieldTraitSpelt
        | Limit::BlockTraitSpelt
        | Limit::FieldElsewhere
        | Limit::TwoFieldsNamed
        | Limit::BodilessInBlock
        | Limit::FromContradicted
        | Limit::IntoRepeated
        | Limit::MarkUnderCfgAttr => String::new(),
    }
}

/// Returns the route a target is reached by, as the variant's option
/// spells it, every step: `.payload`, `.lend()`, `*.payload`; `None` for
/// `deref` alone, which a sentence names as `deref`.
///
/// Open question: no snapshot holds a sentence about a route of several
/// steps; it is read as naming the route as the option spells it.
fn route(target: &TargetDescriptor) -> Option<String> {
    if target.route.is_empty() || target.is_deref() {
        return None;
    }
    let steps: Vec<String> = target
        .route
        .iter()
        .map(|step| match step {
            Step::Deref => String::from("*"),
            Step::Field(name) => format!(".{name}"),
            Step::Method(name) | Step::Yield(name) => format!(".{name}()"),
        })
        .collect();
    Some(steps.concat())
}

/// Returns the names a sentence interpolates for the cell.
fn names(cell: &Cell, limit: Limit, slot: Option<Slot>) -> Vec<Fragment> {
    let method = cell.method_name();
    let target = slot.and_then(|slot| slot.target());
    // A block whose header instantiates the enum names the type it puts in
    // the parameter's place (`tests/ui/instantiated_variant_keeps_its_sentence`).
    let generics = cell.generic.descriptor();
    let instance = generics
        .parameter
        .filter(|_| generics.instantiated)
        .map(|parameter| parameter.instance);
    let stored = slot.zip(target).map(|(slot, (_, target))| {
        let slot = match (slot.parameter, instance) {
            (Some(_), Some(instance)) => Slot {
                parameter: Some(instance),
                ..slot
            },
            (Some(_), None) | (None, _) => slot,
        };
        storage(&slot, &target)
    });
    let variant = slot.map(|slot| named(cell, &slot));
    let mut names = Vec::new();
    // A name that is any of several, which a sentence about one of several
    // variants on one anchor holds.
    let mut any: Option<Fragment> = None;
    // The way out that names one of those variants, as `any` names it.
    let mut arm: Option<Fragment> = None;
    let mut push = |name: Option<String>| names.extend(name.map(Fragment::one));
    match limit {
        Limit::Receiver
        | Limit::OpaqueReturn
        | Limit::StructConversion
        | Limit::TypeDisagreement
        | Limit::ArmsDisagree => {}
        Limit::UnitVariant | Limit::TwoFields => {
            push(variant.clone());
            push(slot.map(|slot| format!("`{}(Payload)`", slot.name())));
        }
        // The option goes on the attribute that implements the trait: the
        // enum's, or a partial block's (#36).
        Limit::SelfArgumentWithoutMismatch => {
            push(Some(format!("{method} takes `Self` arguments")));
            push(Some(String::from(
                match cell.path_descriptor().site == PathSite::Block {
                    true => "`mismatch(m = <expr>)` on this block's attribute",
                    false => "`mismatch(m = <expr>)` on the enum",
                },
            )));
        }
        Limit::SelfInReturn => push(Some(String::from("back into `E`"))),
        Limit::DerefSelfReturn => {
            push(Some(format!("{method} returns `Self`")));
            push(variant.map(|variant| format!("payload of {variant}")));
        }
        Limit::RouteSelfReturn => {
            push(Some(format!("{method} returns `Self`")));
            let route = target.and_then(|(_, target)| route(&target));
            push(
                route
                    .zip(variant)
                    .map(|(route, variant)| format!("the route `{route}` of {variant}")),
            );
        }
        Limit::SelfReturnOtherFields => {
            push(Some(format!("{method} returns `Self`")));
            push(variant.map(|variant| format!("other fields of {variant}")));
        }
        Limit::BoxSelfPlain => {
            push(Some(format!("{method} takes `self: Box<Self>`")));
            push(
                variant
                    .zip(stored)
                    .map(|(variant, stored)| format!("{variant} holds a plain `{stored}`")),
            );
        }
        Limit::NamedFieldUnderWrapper => {
            push(Some(format!("{method} returns `Self` inside")));
            push(Some(String::from("`E` has a named field")));
        }
        // The variant, and the tuple variant a path builds, over the type
        // its field holds as written.
        Limit::NamedVariantUnderWrapper => {
            push(Some(format!("{method} returns `Self` inside")));
            push(variant.map(|variant| format!("{variant} holds its payload in a named field")));
            push(
                slot.zip(stored).map(|(slot, stored)| {
                    format!("write the variant as `{}({stored})`", slot.name())
                }),
            );
        }
        // The option as the field's attribute spells it, by its key.
        Limit::VariantFieldOption => {
            let key = target.and_then(|(_, target)| target.option).map(|option| {
                option
                    .split(|character: char| !character.is_alphanumeric() && character != '_')
                    .next()
                    .unwrap_or(option)
                    .to_owned()
            });
            push(key.map(|key| format!("unknown option `{key}`")));
        }
        Limit::MacroInBlock => {}
        Limit::BoxSelfOnSharedPointer
        | Limit::MovesThroughSharedPointer
        | Limit::MutableBorrowThroughSharedPointer => {
            push(Some(format!("{method} {}", takes(cell, limit))));
            push(
                variant
                    .zip(stored)
                    .map(|(variant, stored)| format!("{variant} dereferences a `{stored}`")),
            );
        }
        Limit::MethodRouteOnMutSelf => {
            push(Some(format!("{method} {}", takes(cell, limit))));
            let route = target.and_then(|(_, target)| route(&target));
            push(
                route
                    .zip(variant)
                    .map(|(route, variant)| format!("the route `{route}` of {variant}")),
            );
        }
        Limit::UnsuppliedType => push(Some(String::from("every variant of `E`"))),
        Limit::DefaultWithoutDefault => {
            push(Some(format!("{method} has no default body in `Probe")))
        }
        // A struct is forwarded to its field, and the sentence speaks of
        // the struct and its field (`inherent_all_with_body_on_a_struct`,
        // #18).
        Limit::BodyNeverRuns => push(Some(match cell.s0.descriptor().is_struct() {
            true => format!("{method} is forwarded to the struct's field"),
            false => format!("{method} is forwarded to every variant"),
        })),
        Limit::ArmNamingNoVariant => push(Some(format!("an arm of {method} names the variants"))),
        Limit::ArmsBesideSelfArgument => push(Some(format!("{method} takes `Self` arguments"))),
        // The trait as the field spells it, the field, and the trait as
        // its home spells it.
        Limit::FieldTraitSpelt => push(cell.s0.home_prefix().map(|prefix| {
            format!("`{TRAIT_ON_FIELD}` on `held` and `{prefix}::{TRAIT_ON_FIELD}` on the struct")
        })),
        Limit::BlockTraitSpelt => push(cell.s0.home_prefix().map(|prefix| {
            format!("`{TRAIT_ON_FIELD}` on `held` and `{prefix}::{TRAIT_ON_FIELD}` here")
        })),
        // The method, and the mark that forwards it in the subject's words
        // (#18).
        Limit::BodilessInBlock => {
            push(Some(format!("{method} has no body")));
            push(Some(String::from(match cell.s0.descriptor().is_struct() {
                true => "forward it to the struct's field with `#[dispatch(*)]`",
                false => "forward it with `#[dispatch(*)]` or `#[dispatch(Variant, ..)]`",
            })));
        }
        // The later option, the earlier one, and the enum.
        Limit::FromContradicted => push(Some(String::from(
            "`from = false` contradicts `from = true` on another attribute of `E`",
        ))),
        Limit::IntoRepeated => push(Some(String::from(
            "`into = support::Sink` is given twice on `E`",
        ))),
        // The method and the two fields it names, in the mark's order.
        Limit::TwoFieldsNamed => push(Some(format!(
            "{method} names two fields, `held` and `label`"
        ))),
        // The field the method names, and the one the trait goes to.
        Limit::FieldElsewhere => push(Some(String::from(
            "`label` is a field of `E`, but this trait is forwarded to `held`",
        ))),
        Limit::ArmsMatchedByPlace => {
            push(Some(String::from("every variant of `E` is under `#[cfg]`")));
            push(Some(format!("so {method} is matched as `match *self`")));
        }
        // Each variant that holds no single payload gets a sentence of its
        // own on the one anchor (`star_over_variants_without_a_payload_
        // beside_a_self_argument`), so a sentence names any of them.
        // Where the method may hold arms of its own, the way out is the
        // variant's arm, its fields left to a rest pattern.
        Limit::StarOverUnforwardable => {
            push(Some(format!("{method} is forwarded to every variant")));
            let held: Vec<Slot> = cell
                .slots()
                .into_iter()
                .filter(|slot| matches!(slot.variant, SlotVariant::Unit | SlotVariant::Pair))
                .collect();
            if cell.takes_own_arms() {
                let arms = held
                    .iter()
                    .map(|slot| {
                        let rest = match slot.variant {
                            SlotVariant::Pair => "(..)",
                            SlotVariant::Unit | SlotVariant::Live(_) => "",
                        };
                        format!("`match self {{ Self::{}{rest} => .. }}`", slot.name())
                    })
                    .collect();
                arm = Some(Fragment(arms));
            }
            let held = held
                .iter()
                .map(|slot| format!("`{}`", slot.name()))
                .collect();
            any = Some(Fragment(held));
        }
        Limit::IntoOtherFields | Limit::RestatedConversions => {}
        // The way around a variant's mark, and a struct's field's, as the
        // snapshots `mark_under_cfg_attr_on_a_variant` and
        // `mark_under_cfg_attr_on_a_struct_field` spell them.
        Limit::MarkUnderCfgAttr => push(Some(String::from(
            match cell.s0.descriptor().is_struct() {
                true => "mark the field in every build, with a bare `#[whichever::dispatch]` on the struct",
                false => "write the variant twice, under `#[cfg(..)]` with the mark and under `#[cfg(not(..))]` without it",
            },
        ))),
        // Open question: whether `into` reports each variant that holds no
        // payload or the first alone is not pinned; a sentence is read as
        // naming any of them.
        Limit::IntoWithoutPayload => {
            let units: Vec<String> = cell
                .slots()
                .into_iter()
                .filter(|slot| slot.variant == SlotVariant::Unit)
                .map(|slot| format!("`{}` holds no payload", slot.name()))
                .collect();
            any = Some(Fragment(units));
        }
        Limit::ConstDisagreement => push(variant.map(|variant| format!("of variant {variant}"))),
        // The parameter and the enum it is a parameter of, as the
        // snapshots name `I` of `Adapter` and `T` of `Kind`.
        Limit::FromOverlapTypeParameter | Limit::TryFromTypeParameter => push(
            slot.and_then(|slot| slot.parameter)
                .map(|parameter| format!("`{parameter}` is a type parameter of `E`")),
        ),
        Limit::MismatchForAbsentMethod => push(Some(format!("`Probe` has no method {method}"))),
        // The sentence is on the method and names the variant that holds
        // the trait object, the mark that leaves the method to the trait's
        // default, and the pointer held without `deref`, which is the way
        // where the pointer implements the trait itself
        // (foreign-traits-are-restated.md: a `Box<dyn Iterator>` does).
        Limit::SizedOnObject => {
            push(Some(format!("{method} requires `Self: Sized`")));
            push(Some(format!(
                "`#[dispatch(default)] fn {}(..);`",
                method.trim_matches('`')
            )));
            push(Some(String::from(
                "hold `Box<dyn Probe>` without `deref` where it implements the trait itself",
            )));
            let objects: Vec<String> = cell
                .method_slots()
                .into_iter()
                .filter(|(_, _, target)| target.is_object())
                .map(|(slot, _, _)| format!("`{}` holds a trait object", slot.name()))
                .collect();
            any = Some(Fragment(objects));
        }
        // The type, its declaration's parameter, and the block it goes in,
        // with the header a block over the enum writes, the lifetime of a
        // subject that borrows included.
        Limit::GenericAssociatedType => {
            push(Some(String::from("`Item` is a generic associated type")));
            // A sentence about a struct speaks of the struct (#18).
            push(Some(match cell.s0.descriptor().is_struct() {
                true => String::from("which only the struct can declare"),
                false => String::from("which only the enum can declare"),
            }));
            push(Some(String::from("`type Item<'v> = ..;`")));
            let borrows = cell
                .live_slots()
                .iter()
                .any(|(_, _, target)| target.borrows());
            // The trait as the subject's attribute spells it.
            let probe = match cell.s0.home_prefix() {
                Some(prefix) => format!("{prefix}::Probe"),
                None => String::from("Probe"),
            };
            push(Some(match borrows {
                true => format!("impl<'a> {probe} for E<'a>` block"),
                false => format!("impl {probe} for E` block"),
            }));
        }
    }
    names.extend(any.filter(|fragment| !fragment.0.is_empty()));
    names.extend(arm.filter(|fragment| !fragment.0.is_empty()));
    names
}
