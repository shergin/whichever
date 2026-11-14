//! The limits table: `docs/limits.md` made executable.
//!
//! A cell's verdict is decided in four passes, each a list of rules:
//!
//! 1. **Malformed**: the trait or a payload's impl is not Rust whatever the
//!    subject. Only the axes that make it so may depart from the neutral
//!    cell; any other departure is non-canonical.
//! 2. **Unpredicted**: the model cannot say what the macro answers, or
//!    cannot write the payloads. The cell is unmodelled.
//! 3. **Limits**: each rule names the sentence the docs promise for a shape
//!    and where it is reported. A cell with any is a limit.
//! 4. **Unwritten**: no limit is predicted, but the model cannot write the
//!    hand or the law. The cell is unmodelled.
//!
//! What is left passes.

use super::argument::{ArgumentKind, MismatchSite};
use super::limits::{Anchor, Expected, Limit};
use super::qualifier::Generic;
use super::receiver::Lends;
use super::returns::{FutureKind, Node};
use super::target::{Pointer, Step, Storage};
use super::{Argument, Qualifier, Receiver, Second, Target, Verdict};
use crate::cell::Cell;

/// Returns the cell's verdict.
pub fn verdict(cell: &Cell) -> Verdict {
    if let Some(reason) = malformed(cell) {
        if !canonical_when_malformed(cell) {
            return Verdict::NonCanonical;
        }
        return Verdict::Malformed(reason);
    }
    if let Some(reason) = unpredicted(cell) {
        return Verdict::Unmodelled(reason);
    }
    let expected = limits(cell);
    if !expected.is_empty() {
        return Verdict::Limit(expected);
    }
    if let Some(reason) = unwritten(cell) {
        return Verdict::Unmodelled(reason);
    }
    Verdict::Pass
}

/// The signature itself, or every payload's impl of it, is ill-typed.
fn malformed(cell: &Cell) -> Option<&'static str> {
    let node = cell.ret.node();
    let lends = cell.recv.descriptor().lends;
    let borrowed_argument = matches!(
        cell.arg.descriptor().kind,
        Some(ArgumentKind::Shared | ArgumentKind::Exclusive)
    );
    if node.borrows_mutably() && lends == Lends::Shared {
        return Some("a `&mut` return under a shared receiver, which no payload can lend");
    }
    if node.borrows() && lends == Lends::Nothing && !borrowed_argument {
        return Some("a reference return under a receiver that holds no lifetime");
    }
    if matches!(node.future(), Some((FutureKind::Boxed, _)))
        && lends == Lends::Nothing
        && !borrowed_argument
    {
        return Some("a boxed future's `'_` under a receiver that holds no lifetime");
    }
    None
}

/// A malformed cell is canonical when only the receiver and the return
/// type, which make it malformed, depart from the neutral cell.
fn canonical_when_malformed(cell: &Cell) -> bool {
    cell.s0 == Target::Plain
        && cell.s1 == Second::Plain
        && cell.arg == Argument::Absent
        && cell.qual == Qualifier::Plain
}

/// The model cannot say what the macro answers, or cannot write the
/// payloads.
fn unpredicted(cell: &Cell) -> Option<&'static str> {
    let node = cell.ret.node();
    let receiver = cell.recv.descriptor();
    let argument = cell.arg.descriptor();
    let qualifier = cell.qual.descriptor();
    let live = cell.live_slots();

    // Open question: neither the docs nor a `tests/ui` case say what a
    // `&mut self` method on a `deref` variant over a shared pointer is. A
    // hand cannot write it; rustc's E0596 is what both would get.
    let exclusive =
        receiver.lends == Lends::Exclusive || argument.kind == Some(ArgumentKind::Exclusive);
    let shared_pointer = live.iter().any(|(_, _, target)| {
        target
            .pointer()
            .is_some_and(|pointer| !pointer.lends_mutably())
    });
    if exclusive && shared_pointer {
        return Some("a mutable borrow through a shared pointer, which no document covers");
    }

    // Open question: `docs/routes.md` says a route ending in a method call
    // serves `self` methods, but a method that lends its target cannot
    // hand it over by value, and no sentence is documented.
    let through_method = live.iter().any(|(_, _, target)| target.ends_in_method());
    let through_loan = live.iter().any(|(_, _, target)| target.ends_in_loan());
    let owned = cell.recv == Receiver::Owned || argument.kind == Some(ArgumentKind::Owned);
    if through_loan && owned {
        return Some("a value moved out of a route that ends in a method call");
    }
    if through_method && argument.kind == Some(ArgumentKind::Exclusive) {
        return Some("a `&mut Self` argument through a route that ends in a method call");
    }

    // Open question: a `Self` argument by value on a `deref` variant over
    // a pointer that does not own is a move out of it; the sentence for
    // `self` names the receiver, and none is documented for an argument.
    let unowned_pointer = live
        .iter()
        .any(|(_, _, target)| target.pointer().is_some_and(|pointer| !pointer.owns()));
    if argument.kind == Some(ArgumentKind::Owned) && unowned_pointer {
        return Some("a `Self` argument moved out of a pointer that does not own it");
    }

    if node.borrows() && receiver.lends == Lends::Nothing {
        return Some("a reference return that borrows from the `Self` argument");
    }
    if argument.kind.is_some() && argument.mismatch != MismatchSite::Missing {
        // Open question: the docs do not say what a mismatch is for a
        // future. The `match` of an opaque future, and of a boxed one whose
        // output is converted, is in one `async move` block, so the
        // mismatch is read as the output there; a boxed future forwarded as
        // it is has no block, so its mismatch is a boxed future itself.
        let output = match node.future() {
            Some((_, output)) => *output,
            None => node,
        };
        // A `Self` in the mismatch is a variant built around a fresh
        // payload, which only the enum's attribute can name, and only for a
        // variant that holds its payload by value.
        let nameable = argument.mismatch == MismatchSite::Enum
            && cell.s0.descriptor().storage == Storage::Payload;
        if !output.has_constant(nameable) {
            return Some("a mismatch expression for a return type that needs a payload to make");
        }
    }
    if qualifier.asyncness && node.holds_future() {
        return Some("an `async fn` that returns a future");
    }
    if qualifier.provided && !node.has_default() {
        return Some("a default body that would need a payload to make its answer");
    }
    None
}

/// The sentences the docs promise for the cell, each with where it is
/// reported.
fn limits(cell: &Cell) -> Vec<Expected> {
    let mut expected = Vec::new();
    let mut expect = |limit, anchor| {
        let entry = Expected { limit, anchor };
        if !expected.contains(&entry) {
            expected.push(entry);
        }
    };
    let node = cell.ret.node();
    let receiver = cell.recv.descriptor();
    let argument = cell.arg.descriptor();
    let slots = cell.slots();
    let live = cell.live_slots();

    // "Receivers are `self`, `&self`, `&mut self`, `self: Box<Self>` [..],
    // and `self: Pin<&Self>` or `self: Pin<&mut Self>` [..]. `Rc<Self>`,
    // `Arc<Self>` and `Pin<Box<Self>>` would need a refcount or an
    // allocation, so they are not."
    if receiver.unwrap.is_none() {
        expect(Limit::Receiver, Anchor::Method);
    }

    // "Every variant a trait is dispatched to holds exactly one unnamed
    // field, `Name(Payload)`, or marks the field to forward to with
    // `#[dispatch]`."
    for slot in &slots {
        match slot.variant {
            super::SlotVariant::Live(_) => {}
            super::SlotVariant::Unit => expect(Limit::UnitVariant, Anchor::Variant(slot.index)),
            super::SlotVariant::Pair => expect(Limit::TwoFields, Anchor::Variant(slot.index)),
        }
    }

    // `mismatch` in docs/options.md: a `Self` argument needs the result
    // for two different variants, which one variant never meets.
    if argument.kind.is_some() && argument.mismatch == MismatchSite::Missing && slots.len() > 1 {
        expect(Limit::SelfArgumentWithoutMismatch, Anchor::Method);
    }

    // "An enum cannot dispatch a return-position `impl Trait`, at any depth
    // of the return type, except `impl Future<Output = T>`."
    let opaque = matches!(node, Node::Iterator);
    if opaque {
        expect(Limit::OpaqueReturn, Anchor::Method);
    }

    // "A method whose return type wraps `Self` in anything but `Option`,
    // `Result` or a tuple [..] cannot be forwarded", nor a tuple under an
    // `Option` (what-is-generated.md).
    let conversion = node.conversion();
    if conversion.is_none() && !opaque {
        expect(Limit::SelfInReturn, Anchor::Method);
    }

    // "A variant reached through a route, `deref` included, cannot take
    // part in a method returning `Self`."
    //
    // The marked field of a variant with several gets the sentence of a
    // struct's marked field, `struct_self_return_other_fields`, since the
    // rebuild would drop the other fields (docs/structs.md); a marked field
    // with a route of its own gets the route's. Open question:
    // `docs/limits.md` and `tests/ui` hold the struct's case only.
    if conversion.is_some() && node.holds_this() {
        for (slot, _, target) in &live {
            let anchor = Anchor::Variant(slot.index);
            if target.pointer().is_some() {
                expect(Limit::DerefSelfReturn, anchor);
            } else if target.is_routed() {
                expect(Limit::RouteSelfReturn, anchor);
            } else if target.has_other_fields() {
                expect(Limit::SelfReturnOtherFields, anchor);
            }
        }
    }

    // "`self: Box<Self>` on `deref` variants holding a `Box`"; a plain
    // variant is an error, since boxing it again would allocate.
    //
    // Open question: a variant over another pointer, or reached through a
    // field, is read as getting the same sentence.
    if cell.recv == Receiver::Boxed {
        for (slot, _, target) in &live {
            let boxed = target.pointer() == Some(Pointer::Box) && target.route == [Step::Deref];
            if !boxed {
                expect(Limit::BoxSelfPlain, Anchor::Variant(slot.index));
            }
        }
    }

    // "A value cannot be moved out of a `deref` variant that holds
    // anything but a `Box`: a method taking `self` is reported on the
    // payload."
    if cell.recv == Receiver::Owned {
        for (slot, _, target) in &live {
            if target.pointer().is_some_and(|pointer| !pointer.owns()) {
                expect(
                    Limit::MovesThroughSharedPointer,
                    Anchor::Variant(slot.index),
                );
            }
        }
    }

    // "A route ending in a method call cannot serve a `&mut self` method."
    //
    // Open question: `self: Pin<&mut Self>` is read as a `&mut self`
    // method here.
    if receiver.lends == Lends::Exclusive {
        for (slot, _, target) in &live {
            if target.ends_in_method() {
                expect(Limit::MethodRouteOnMutSelf, Anchor::Variant(slot.index));
            }
        }
    }

    expected.sort();
    expected
}

/// No limit is predicted, but the model cannot write the hand or the law.
fn unwritten(cell: &Cell) -> Option<&'static str> {
    let node = cell.ret.node();
    let qualifier = cell.qual.descriptor();

    // A boxed future whose output is converted is one `async move` block
    // that captures every argument, and `X` is not bound to outlive `'_`:
    // rustc's E0311 is what the hand gets, on the trait's tokens as the
    // macro does. Open question: no verdict says "the hand fails the same
    // way" yet.
    let converted = node.future().is_some_and(|(kind, output)| {
        kind == FutureKind::Boxed
            && output
                .conversion()
                .is_some_and(|conversion| !conversion.is_keep())
    });
    if converted && qualifier.generic == Some(Generic::Fixed) {
        return Some(
            "a converted boxed future that captures a type parameter without a lifetime bound",
        );
    }

    // The same block captures a borrowed `Self` argument, whose lifetime is
    // not the `'_` of the boxed future: "lifetime may not live long enough"
    // is what the hand gets.
    let borrowed_argument = matches!(
        cell.arg.descriptor().kind,
        Some(ArgumentKind::Shared | ArgumentKind::Exclusive)
    );
    if converted && borrowed_argument {
        return Some(
            "a converted boxed future that captures a `Self` argument of another lifetime",
        );
    }

    // A route ending in a method that returns its target by value hands a
    // temporary over, which an answer cannot borrow from: the hand would
    // return a reference to it, or a boxed future holding one.
    let temporary = cell
        .live_slots()
        .iter()
        .any(|(_, _, target)| target.ends_in_temporary());
    let lends = cell.recv.descriptor().lends != Lends::Nothing;
    let boxed = matches!(node.future(), Some((FutureKind::Boxed, _)));
    if temporary && lends && (node.borrows() || (boxed && !converted)) {
        return Some("an answer that borrows a temporary the route's method returns");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Return;

    #[test]
    fn the_neutral_cell_passes() {
        assert_eq!(verdict(&Cell::neutral()), Verdict::Pass);
    }

    #[test]
    fn rejected_receivers_are_limits_on_the_method() {
        for receiver in [Receiver::Rc, Receiver::Arc, Receiver::PinBoxed] {
            let cell = Cell {
                recv: receiver,
                ..Cell::neutral()
            };
            assert_eq!(
                verdict(&cell),
                Verdict::Limit(vec![Expected {
                    limit: Limit::Receiver,
                    anchor: Anchor::Method
                }])
            );
        }
    }

    #[test]
    fn a_mutable_return_under_a_shared_receiver_is_malformed() {
        let cell = Cell {
            ret: Return::Exclusive,
            ..Cell::neutral()
        };
        assert!(matches!(verdict(&cell), Verdict::Malformed(_)));
        let departed = Cell {
            s0: Target::DerefBox,
            ..cell
        };
        assert_eq!(verdict(&departed), Verdict::NonCanonical);
    }
}
