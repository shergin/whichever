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

/// Returns the fragments a sentence the model predicts has to hold, each
/// in full.
pub fn fragments(cell: &Cell, expected: &Expected) -> Vec<String> {
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
    let body_for_the_rest = cell.path.descriptor().form == PathForm::First;
    let rebuild_left_to_the_body =
        expected.limit == Limit::SelfReturnOtherFields && body_for_the_rest;
    let mut fragments = Vec::new();
    if !rebuild_left_to_the_body {
        fragments.push(String::from(entry.way_out));
    }
    if entry.tail || rebuild_left_to_the_body {
        if let Some(tail) = tail(cell, variant.as_deref()) {
            fragments.push(tail);
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
/// and by hand for a trait's method, a block's mark included. `None` where
/// the snapshots say nothing.
///
/// Open question: a sentence on the whole method of a method that has a
/// body is not pinned by a snapshot, so its tail is not held.
fn tail(cell: &Cell, variant: Option<&str>) -> Option<String> {
    let path = cell.path.descriptor();
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
        (Tail::LeaveToBody, None) => None,
        (Tail::LeaveToBody, Some(variant)) => Some(tail.fragment(variant)),
        (Tail::ByHand | Tail::Body, _) => Some(tail.fragment("")),
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
            let inherent = cell.path.descriptor().site == PathSite::Inherent;
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
        | Limit::TypeDisagreement
        | Limit::ConstDisagreement
        | Limit::MismatchForAbsentMethod => String::new(),
    }
}

/// Returns the route a target is reached by, as the variant's option
/// spells it: `.payload`, `.lend()`.
fn route(target: &TargetDescriptor) -> Option<String> {
    let step = target.route.last()?;
    match step {
        Step::Deref => None,
        Step::Field(name) => Some(format!(".{name}")),
        Step::Method(name) | Step::Yield(name) => Some(format!(".{name}()")),
    }
}

/// Returns the names a sentence interpolates for the cell.
fn names(cell: &Cell, limit: Limit, slot: Option<Slot>) -> Vec<String> {
    let method = "`m`";
    let target = slot.and_then(|slot| slot.target());
    let stored = slot
        .zip(target)
        .map(|(slot, (_, target))| storage(&slot, &target));
    let variant = slot.map(|slot| named(cell, &slot));
    let mut names = Vec::new();
    let mut push = |name: Option<String>| names.extend(name);
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
        Limit::StarOverUnforwardable => {
            push(Some(format!("{method} is forwarded to every variant")));
            let held = cell
                .slots()
                .into_iter()
                .find(|slot| matches!(slot.variant, SlotVariant::Unit | SlotVariant::Pair));
            push(held.map(|slot| format!("`{}`", slot.name())));
        }
        Limit::IntoOtherFields => {}
        Limit::IntoWithoutPayload => {
            let unit = cell
                .slots()
                .into_iter()
                .find(|slot| slot.variant == SlotVariant::Unit);
            push(unit.map(|slot| format!("`{}` holds no payload", slot.name())));
        }
        Limit::ConstDisagreement => push(variant.map(|variant| format!("of variant {variant}"))),
        Limit::MismatchForAbsentMethod => push(Some(format!("`Probe` has no method {method}"))),
    }
    names
}
