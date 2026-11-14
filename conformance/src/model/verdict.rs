//! The limits table: `docs/limits.md` made executable.
//!
//! A cell's verdict is decided in passes, each a list of rules:
//!
//! 1. **Malformed**: the trait or a payload's impl is not Rust whatever the
//!    subject. Only the axes that make it so may depart from the neutral
//!    cell; any other departure is non-canonical.
//! 2. **Unpredicted**: the model cannot say what the macro answers, or
//!    cannot write the payloads. The cell is unmodelled.
//! 3. **Limits**: each rule names the sentence the docs promise for a shape
//!    and where it is reported.
//! 4. **Rejections**: each rule names a forwarding no hand can write, with
//!    the rustc error the hand gets, where no sentence is documented.
//!
//! A cell with a limit is a limit when a limit of `m` replaces the body a
//! rejection of `m` would have; a cell with a rejection and no limit is
//! rejected; what is left passes.

use super::argument::{ArgumentKind, MismatchSite};
use super::limits::{Anchor, Expected, Limit};
use super::path::{Form as PathForm, Site as PathSite};
use super::qualifier::Generic;
use super::receiver::Lends;
use super::returns::{FutureKind, Node};
use super::target::{Pointer, Step, Storage};
use super::{Argument, Qualifier, Receiver, Return, Second, Target, Verdict};
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
    let rejections = rejections(cell);
    if !expected.is_empty() {
        // A method that cannot be forwarded is written with its sentence as
        // its body (macros/ARCHITECTURE.md 3.8), so a sentence on `m`
        // stands for any rejection of `m`; a rejection of the control
        // method, or one beside a sentence on a variant only, would be a
        // second error the model does not predict yet.
        let on_method = expected
            .iter()
            .any(|expected| expected.limit.is_of_method());
        let covered = rejections
            .iter()
            .all(|rejection| on_method && rejection.method == Forwarded::Method);
        if !covered {
            return Verdict::Unmodelled("a limit beside a rejection the limit does not replace");
        }
        return Verdict::Limit(expected);
    }
    if let Some(rejection) = rejections.first() {
        return Verdict::Rejected {
            code: rejection.code,
            reason: rejection.reason,
        };
    }
    Verdict::Pass
}

/// Which forwarded method a rejection concerns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Forwarded {
    /// The method under test.
    Method,
    /// The control method.
    Base,
}

/// A forwarding no hand can write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rejection {
    code: &'static str,
    reason: &'static str,
    method: Forwarded,
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
        && cell.path == super::Path::Attribute
}

/// The model cannot say what the macro answers, or cannot write the
/// payloads.
fn unpredicted(cell: &Cell) -> Option<&'static str> {
    let node = cell.ret.node();
    let receiver = cell.recv.descriptor();
    let argument = cell.arg.descriptor();
    let qualifier = cell.qual.descriptor();

    if node.borrows() && receiver.lends == Lends::Nothing {
        return Some("a reference return that borrows from the `Self` argument");
    }
    let path = cell.path.descriptor();
    if path.site == PathSite::Inherent {
        // An inherent method forwards by method-call syntax to the
        // payloads' own `m` (inherent-methods.md); the model covers the
        // receivers, returns and qualifiers that page describes for an
        // enum.
        if !matches!(
            cell.recv,
            Receiver::Shared | Receiver::Exclusive | Receiver::Owned
        ) {
            return Some(
                "an inherent method with a receiver other than `&self`, `&mut self` or `self`",
            );
        }
        if cell.arg != Argument::Absent {
            return Some("an inherent method with a `Self` argument, which is the enum itself");
        }
        if !matches!(cell.qual, Qualifier::Plain | Qualifier::Async) {
            return Some("an inherent method that is generic, unsafe or provided");
        }
        let supported = match cell.ret {
            Return::Tag
            | Return::Unit
            | Return::Shared
            | Return::Exclusive
            | Return::This
            | Return::OptionThis
            | Return::ResultThis
            | Return::Future => true,
            Return::TupleThis
            | Return::Assoc
            | Return::Out
            | Return::OptionOut
            | Return::ResultError
            | Return::ResultThisError
            | Return::FutureThis
            | Return::BoxedFuture
            | Return::BoxedFutureOut
            | Return::Iterator
            | Return::VecThis
            | Return::OptionTupleThis => false,
        };
        if !supported {
            return Some("an inherent method with a return type the inherent docs do not describe for an enum");
        }
        let routed_self = node.holds_this()
            && cell
                .method_slots()
                .iter()
                .any(|(_, _, target)| target.is_routed() || target.has_other_fields());
        if routed_self {
            return Some(
                "an inherent `Self` return on a variant reached through a route or a marked field",
            );
        }
        let method_route = cell
            .method_slots()
            .iter()
            .any(|(_, _, target)| target.ends_in_method());
        if method_route && cell.recv != Receiver::Shared {
            return Some("an inherent method through a route ending in a method call under a receiver other than `&self`");
        }
        let lifetime = cell
            .live_slots()
            .iter()
            .any(|(_, _, target)| target.borrows());
        if cell.ret == Return::Future && lifetime {
            return Some("an inherent opaque future on an enum with a lifetime, whose capture needs `use<..>`");
        }
        if cell
            .method_slots()
            .iter()
            .any(|(_, _, target)| target.ends_in_temporary())
        {
            return Some("an inherent method through a route whose method yields its target");
        }
    }
    // A body a block writes, for the variants a mark leaves out or for
    // every variant, is a constant of the return type.
    if matches!(path.form, PathForm::First | PathForm::Written) {
        if node.holds_future() {
            return Some("a block body for a future return");
        }
        let nameable = cell.s0.descriptor().storage == Storage::Payload;
        if !node.has_constant(nameable) {
            return Some("a block body for a return type that needs a payload to make");
        }
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
    let yielded = cell
        .live_slots()
        .iter()
        .all(|(_, _, target)| target.ends_in_temporary());
    if qualifier.provided && yielded {
        return Some("a default body that calls a control method taking `self`");
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
    let path = cell.path.descriptor();
    // The variant-specific rules of `m` concern the variants it is
    // forwarded to, and the method-wide ones apply when it is forwarded at
    // all: a method a block writes, or leaves to the default, is no
    // forwarding.
    let method_live = cell.method_slots();
    let forwarded = matches!(path.form, PathForm::Forwarded | PathForm::First);

    // A sentence on the whole method sits on its signature in the block
    // when the block holds it, and on the trait's method otherwise.
    let on_method = match path.in_block {
        true => Anchor::BlockSignature,
        false => Anchor::Method,
    };

    // A mark naming every variant leaves no variant for the body, and `*`
    // names a variant that holds no payload too (partial-impl-blocks.md,
    // after inherent-methods.md: `inherent_all_with_body`,
    // `inherent_all_with_unforwardable`).
    let unreachable = slots.iter().any(|slot| slot.target().is_none());
    if path.form == PathForm::First && slots.len() == 1 {
        expect(Limit::BodyNeverRuns, Anchor::BlockSignature);
    }
    if path.mark == Some("*") && unreachable {
        expect(Limit::StarOverUnforwardable, Anchor::BlockSignature);
    }

    // "`#[dispatch(default)]` on a bodiless method leaves it to the trait's
    // default" (partial-impl-blocks.md), which a required method has none of.
    if path.form == PathForm::Default && !cell.qual.descriptor().provided {
        expect(Limit::DefaultWithoutDefault, Anchor::BlockMethod);
    }

    // "Receivers are `self`, `&self`, `&mut self`, `self: Box<Self>` [..],
    // and `self: Pin<&Self>` or `self: Pin<&mut Self>` [..]. `Rc<Self>`,
    // `Arc<Self>` and `Pin<Box<Self>>` would need a refcount or an
    // allocation, so they are not."
    if forwarded && receiver.unwrap.is_none() {
        expect(Limit::Receiver, on_method);
    }

    // "Every variant a trait is dispatched to holds exactly one unnamed
    // field, `Name(Payload)`, or marks the field to forward to with
    // `#[dispatch]`." A block method that names the variants it forwards
    // to is how a trait reaches the others, the control method included.
    for slot in slots.iter().filter(|_| cell.base_reaches_all()) {
        match slot.variant {
            super::SlotVariant::Live(_) => {}
            super::SlotVariant::Unit => expect(Limit::UnitVariant, Anchor::Variant(slot.index)),
            super::SlotVariant::Pair => expect(Limit::TwoFields, Anchor::Variant(slot.index)),
        }
    }

    // `mismatch` in docs/options.md: a `Self` argument needs the result
    // for two different variants, which one variant never meets.
    if path.form == PathForm::Forwarded
        && argument.kind.is_some()
        && argument.mismatch == MismatchSite::Missing
        && slots.len() > 1
    {
        expect(Limit::SelfArgumentWithoutMismatch, Anchor::Method);
    }

    // "An enum cannot dispatch a return-position `impl Trait`, at any depth
    // of the return type, except `impl Future<Output = T>`."
    let opaque = forwarded && matches!(node, Node::Iterator);
    if opaque {
        expect(Limit::OpaqueReturn, on_method);
    }

    // "A method whose return type wraps `Self` in anything but `Option`,
    // `Result` or a tuple [..] cannot be forwarded", nor a tuple under an
    // `Option` (what-is-generated.md).
    let conversion = node.conversion();
    if forwarded && conversion.is_none() && !matches!(node, Node::Iterator) {
        expect(Limit::SelfInReturn, on_method);
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
        for (slot, _, target) in &method_live {
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
        for (slot, _, target) in &method_live {
            let boxed = target.pointer() == Some(Pointer::Box) && target.route == [Step::Deref];
            if !boxed {
                expect(Limit::BoxSelfPlain, Anchor::Variant(slot.index));
            }
        }
    }

    // "A value cannot be moved out of a `deref` variant that holds
    // anything but a `Box`: a method taking `self` is reported on the
    // payload."
    //
    // A `Self` argument by value gets the same sentence, which names the
    // receiver though it is the argument that would move.
    if cell.recv == Receiver::Owned || argument.kind == Some(ArgumentKind::Owned) {
        for (slot, _, target) in &method_live {
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
    // `self: Pin<&mut Self>` and a `&mut Self` argument get the same
    // sentence, which names `&mut self` for both.
    if receiver.lends == Lends::Exclusive || argument.kind == Some(ArgumentKind::Exclusive) {
        for (slot, _, target) in &method_live {
            if target.ends_in_method() {
                expect(Limit::MethodRouteOnMutSelf, Anchor::Variant(slot.index));
            }
        }
    }

    // `type X;` becomes `type X = <FirstVariant as Trait>::X;`
    // (what-is-generated.md), and a variant whose target has no nameable
    // type takes no part (routes.md): with no variant whose target is
    // named, the sentence of `all_variants_conditional` is on the enum.
    let taken = node.associated().iter().any(|(_, overridden)| !overridden);
    let named = live
        .iter()
        .any(|(_, _, target)| target.route.iter().all(|step| *step == Step::Deref));
    if taken && !named {
        expect(Limit::UnsuppliedType, Anchor::Enum);
    }

    expected.sort();
    expected
}

/// The forwardings no hand can write, each with the rustc error the hand
/// gets. Where the macro answers such a shape with a sentence, the shape is
/// a limit rule instead.
fn rejections(cell: &Cell) -> Vec<Rejection> {
    let mut rejections = Vec::new();
    let mut reject = |code, reason, method| {
        rejections.push(Rejection {
            code,
            reason,
            method,
        })
    };
    let node = cell.ret.node();
    let receiver = cell.recv.descriptor();
    let argument = cell.arg.descriptor();
    let qualifier = cell.qual.descriptor();
    let live = cell.method_slots();
    let forwarded = !live.is_empty();
    let any = |predicate: &dyn Fn(&super::target::TargetDescriptor) -> bool| {
        live.iter().any(|(_, _, target)| predicate(target))
    };

    // `deref` dispatches `Rc<T>`, `Arc<T>` and `&T` "like a plain `T`"
    // (docs/options.md), but nothing can be borrowed mutably through them:
    // the hand's `&mut **inner` is E0596, and no sentence is documented.
    let exclusive =
        receiver.lends == Lends::Exclusive || argument.kind == Some(ArgumentKind::Exclusive);
    if exclusive
        && any(&|target| {
            target
                .pointer()
                .is_some_and(|pointer| !pointer.lends_mutably())
        })
    {
        reject(
            "E0596",
            "a mutable borrow through a pointer that lends only shared access",
            Forwarded::Method,
        );
    }

    // A route ending in a method call hands the method's result over as it
    // is (docs/decisions/an-arm-reaches-its-target-through-a-route.md): a
    // method that lends serves the receivers that take a reference, one
    // that yields the target by value serves `self` by value. The other
    // pairing is a type error the hand's call gets as well.
    let owned = cell.recv == Receiver::Owned || argument.kind == Some(ArgumentKind::Owned);
    if any(&|target| target.ends_in_loan()) && owned {
        reject(
            "E0308",
            "a lent target handed to a method that takes it by value",
            Forwarded::Method,
        );
    }
    let borrowed = matches!(
        argument.kind,
        Some(ArgumentKind::Shared | ArgumentKind::Exclusive)
    );
    if any(&|target| target.ends_in_temporary()) && (receiver.lends == Lends::Shared || borrowed) {
        reject(
            "E0308",
            "a yielded target handed to a method that takes a reference",
            Forwarded::Method,
        );
    }
    let base_slots = cell.base_slots();
    let some_yield = base_slots
        .iter()
        .any(|(_, _, target)| target.ends_in_temporary());
    let every_yield = base_slots
        .iter()
        .all(|(_, _, target)| target.ends_in_temporary());
    if some_yield && !every_yield {
        reject(
            "E0308",
            "a yielded target handed to the control method, which takes `&self`",
            Forwarded::Base,
        );
    }

    // A boxed future whose output is converted is one `async move` block
    // that captures every argument. `X` is not bound to outlive the
    // future's `'_`, which is E0311, and a borrowed `Self` argument has a
    // lifetime of its own beside the receiver's; under a receiver that
    // holds none, `'_` is the argument's.
    let converted = node.future().is_some_and(|(kind, output)| {
        kind == FutureKind::Boxed
            && output
                .conversion()
                .is_some_and(|conversion| !conversion.is_keep())
    });
    if forwarded && converted && qualifier.generic == Some(Generic::Fixed) {
        reject(
            "E0311",
            "a converted boxed future that captures a type parameter without a lifetime bound",
            Forwarded::Method,
        );
    }
    if forwarded && converted && borrowed && receiver.lends != Lends::Nothing {
        reject(
            "lifetime may not live long enough",
            "a converted boxed future that captures a `Self` argument of another lifetime",
            Forwarded::Method,
        );
    }
    rejections
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
