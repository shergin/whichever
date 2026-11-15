//! What a sentence has to say for a cell beside its needle: the way out
//! its catalogue entry names, the method's own way out where it ends in
//! one, and the names it interpolates, computed from the cell. The
//! wording is the snapshots' in `tests/ui`.

use super::argument::ArgumentKind;
use super::limits::{Anchor, Expected, Limit, Tail};
use super::path::{Form as PathForm, Site as PathSite};
use super::receiver::Lends;
use super::target::{Step, TargetDescriptor};
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
        | Anchor::Anywhere => None,
    };
    let variant = slot.map(|slot| named(cell, &slot));
    // The way out of a rebuild that would drop the other fields is the
    // whole of the sentence's advice: where the method names its variants
    // and has a body for the rest, it is that body
    // (`self_return_beside_the_other_fields_of_a_variant` holds the other
    // form), and the entry's own way out is not written.
    let body_for_the_rest = cell.path_descriptor().form == PathForm::First;
    let rebuild_left_to_the_body =
        expected.limit == Limit::SelfReturnOtherFields && body_for_the_rest;
    let mut fragments = Vec::new();
    if !rebuild_left_to_the_body {
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
/// mark included.
fn tail(cell: &Cell, variant: Option<&str>) -> Option<String> {
    let path = cell.path_descriptor();
    let tail = match (path.form, path.site) {
        (PathForm::First, _) => Tail::LeaveToBody,
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
        (Tail::ByHand | Tail::Body | Tail::DropMark, _) => Some(tail.fragment("")),
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
        | Limit::MismatchForAbsentMethod => String::new(),
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
    let stored = slot
        .zip(target)
        .map(|(slot, (_, target))| storage(&slot, &target));
    let variant = slot.map(|slot| named(cell, &slot));
    let mut names = Vec::new();
    // A name that is any of several, which a sentence about one of several
    // variants on one anchor holds.
    let mut any: Option<Fragment> = None;
    let mut push = |name: Option<String>| names.extend(name.map(Fragment::one));
    match limit {
        Limit::Receiver
        | Limit::OpaqueReturn
        | Limit::StructConversion
        | Limit::TypeDisagreement => {}
        Limit::UnitVariant | Limit::TwoFields => {
            push(variant.clone());
            push(slot.map(|slot| format!("`{}(Payload)`", slot.name())));
        }
        Limit::SelfArgumentWithoutMismatch => {
            push(Some(format!("{method} takes `Self` arguments")));
            push(Some(String::from("`mismatch(m = <expr>)`")));
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
        Limit::BodyNeverRuns => push(Some(format!("{method} is forwarded to every variant"))),
        // Each variant that holds no single payload gets a sentence of its
        // own on the one anchor (`star_over_variants_without_a_payload_
        // beside_a_self_argument`), so a sentence names any of them.
        Limit::StarOverUnforwardable => {
            push(Some(format!("{method} is forwarded to every variant")));
            let held: Vec<String> = cell
                .slots()
                .into_iter()
                .filter(|slot| matches!(slot.variant, SlotVariant::Unit | SlotVariant::Pair))
                .map(|slot| format!("`{}`", slot.name()))
                .collect();
            any = Some(Fragment(held));
        }
        Limit::IntoOtherFields | Limit::RestatedConversions => {}
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
        Limit::MismatchForAbsentMethod => push(Some(format!("`Probe` has no method {method}"))),
    }
    names.extend(any.filter(|fragment| !fragment.0.is_empty()));
    names
}
