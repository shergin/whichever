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

use super::agreement::ItemKind;
use super::argument::{ArgumentKind, MismatchSite};
use super::limits::{Anchor, Expected, Limit};
use super::path::{Form as PathForm, Site as PathSite};
use super::qualifier::Generic;
use super::receiver::Lends;
use super::returns::{Associated, FutureKind, Node};
use super::target::{Form, Pointer, Storage};
use super::{
    Agreement, Argument, Arms, Callee, Cfg, Conversion, Declaration, Deprecation, Generics, Home,
    Item, Lift, Owner, Qualifier, Receiver, Return, Second, Stack, Target, Third, Verdict,
};
use crate::cell::Cell;

/// Returns the cell's verdict.
pub fn verdict(cell: &Cell) -> Verdict {
    // The callee and the lift are an inherent method's alone.
    let inherent = cell.path_descriptor().site == PathSite::Inherent;
    if !inherent && (cell.callee != Callee::Method || cell.lift != Lift::Bare) {
        return Verdict::NonCanonical;
    }
    // The attributes are on the trait's method, which an inherent method
    // is not.
    if inherent && cell.item != Item::Bare {
        return Verdict::NonCanonical;
    }
    // Nor has an inherent method a mismatch or a default body: its `Self`
    // argument is the enum, handed over as it is (inherent-methods.md), so
    // `&Self` is spelled once, with no mismatch, and a provided method is a
    // trait's.
    let mismatched = matches!(cell.arg, Argument::SharedOnTrait | Argument::SharedOnEnum);
    if inherent && (mismatched || cell.qual == Qualifier::Provided) {
        return Verdict::NonCanonical;
    }
    // The generics are the trait's and its impl's, which an inherent
    // method's arms do not name.
    if inherent && cell.generic != Generics::Bare {
        return Verdict::NonCanonical;
    }
    // The conversions are options of the enum's own attribute; the model
    // checks them where it names the trait, on a subject of no generics or
    // of a type-parameter payload, whose `From` and `TryFrom` the docs
    // bound (`from_overlap_type_parameter`, `try_from_type_parameter`).
    let converted = cell.conv != Conversion::Unchecked;
    let parameter_conversion = matches!(
        cell.conv,
        Conversion::From | Conversion::FromOff | Conversion::Try | Conversion::VariantFromOff
    );
    let generic = match cell.generic {
        Generics::Bare => false,
        Generics::Payload => !parameter_conversion,
        Generics::Concrete
        | Generics::Own
        | Generics::Defaulted
        | Generics::Paired
        | Generics::Lifetime
        | Generics::Every
        | Generics::Instance => true,
    };
    if converted && (cell.path != super::Path::Attribute || generic) {
        return Verdict::NonCanonical;
    }
    // A struct subject has its one field and no second variant.
    let is_struct = cell.s0.descriptor().is_struct();
    let later = cell.s1 != Second::Absent || cell.s2 != Third::Absent;
    if is_struct && (later || cell.cfg != Cfg::Absent) {
        return Verdict::NonCanonical;
    }
    // A third variant stands beside a second; without one it is a second
    // variant by another name.
    if cell.s1 == Second::Absent && cell.s2 != Third::Absent {
        return Verdict::NonCanonical;
    }
    // A deprecation is a variant's, and the second exists only beside a
    // second variant.
    let deprecated_absent = cell.deprecated == Deprecation::Second && cell.s1 == Second::Absent;
    if deprecated_absent || (is_struct && cell.deprecated != Deprecation::Absent) {
        return Verdict::NonCanonical;
    }
    // The method returning the trait's own type is the dispatchable trait's;
    // a restatement, or a restated enum's real trait, would list it too.
    let returned = cell.agree.descriptor().returned;
    if returned && (cell.decl != Declaration::Dispatchable || cell.owner != Owner::Local) {
        return Verdict::NonCanonical;
    }
    if !object_canonical(cell) {
        return Verdict::NonCanonical;
    }
    if let Some(verdict) = own_arms(cell) {
        return verdict;
    }
    // A block's own header is how the impl gets generics the enum does not
    // have, or one instantiation of a generic enum (partial-impl-blocks.md,
    // "The impl header"), which the enum's attribute writes neither of.
    let header = matches!(cell.generic, Generics::Every | Generics::Instance);
    if header && cell.path_descriptor().site != PathSite::Block {
        return Verdict::NonCanonical;
    }
    // A generic associated type left undeclared is a partial block's way of
    // leaving it out; nothing else declares it either way.
    if cell.ret == Return::GatUndeclared && cell.path_descriptor().site != PathSite::Block {
        return Verdict::NonCanonical;
    }
    if let Some(verdict) = unread(cell) {
        return verdict;
    }
    if let Some(verdict) = declared(cell) {
        return verdict;
    }
    if let Some(verdict) = owned(cell) {
        return verdict;
    }
    if let Some(reason) = malformed(cell) {
        if !canonical_when_malformed(cell) {
            return Verdict::NonCanonical;
        }
        return Verdict::Malformed(reason);
    }
    if let Some(reason) = unpredicted(cell) {
        return Verdict::Unmodelled(reason);
    }
    let mut expected = limits(cell);
    let mut rejections = rejections(cell);
    // `whichever::foreign::Display` has no control method.
    if cell.decl.descriptor().foreign {
        rejections.retain(|rejection| rejection.method == Forwarded::Method);
    }
    // A method the trait declares under a `#[cfg]` that does not hold has
    // no generated item in this build ("the generated item exists exactly
    // when the trait's does", what-is-generated.md), so nothing about it
    // is reported or rejected; what concerns the variants through the
    // control method stands.
    if !cell.method_present() {
        expected.retain(|expected| !expected.limit.is_of_method());
        rejections.retain(|rejection| rejection.method == Forwarded::Base);
    }
    // A sentence that replaces the method is its whole body, so no arm of
    // it is written to carry a sentence about a variant.
    if expected
        .iter()
        .any(|expected| expected.limit.replaces_method())
    {
        expected
            .retain(|expected| expected.limit.replaces_method() || !expected.limit.is_of_method());
    }
    if !expected.is_empty() {
        // A method that cannot be forwarded is written with its sentence as
        // its body (macros/ARCHITECTURE.md 3.8), so a sentence on `m`
        // stands for any rejection of `m`. A sentence about a variant is
        // that variant's arm, and the other arms are type-checked beside
        // it, so a rejection there shows as it does for a hand; that pair,
        // and a rejection of the control method, are not modelled yet.
        let on_method = expected
            .iter()
            .any(|expected| expected.limit.replaces_method());
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

/// The shapes the arms of a method under `*` take, or `None` where the
/// cell is judged as any other. The `arms` axis is the body's, which only
/// the paths that give one under `*` have; on a struct, which has no
/// variant for an arm to name, any body is its sentence, so its neutral
/// value stands for every one.
fn own_arms(cell: &Cell) -> Option<Verdict> {
    let arms_path = cell.path_descriptor().form == PathForm::Arms;
    if !arms_path {
        return (cell.arms != Arms::Answered).then_some(Verdict::NonCanonical);
    }
    if cell.s0.descriptor().is_struct() {
        return (cell.arms != Arms::Answered).then_some(Verdict::NonCanonical);
    }
    // The arms name the second variant: an enum of one has none to name.
    if cell.s1 == Second::Absent {
        return Some(Verdict::Unmodelled(
            "arms of the method's own over an enum of one variant",
        ));
    }
    // Naming the second variant alone differs from naming every variant
    // without a payload only beside a third that holds none.
    if cell.arms == Arms::Second && cell.s2 != Third::Unit {
        return Some(Verdict::NonCanonical);
    }
    // An arm's own `#[cfg]` is crossed with a variant under none, so that
    // the arm alone decides whether the variant is forwarded.
    let conditional = matches!(cell.arms, Arms::CfgOn | Arms::CfgOff);
    if conditional && cell.cfg.of(1).is_some() {
        return Some(Verdict::NonCanonical);
    }
    // `whichever::foreign::Display`'s `fmt` is written by the model as
    // the trait's attribute or a block names it, with no arms of its own.
    if cell.decl.descriptor().foreign {
        return Some(Verdict::NonCanonical);
    }
    // In a build that leaves no variant, no arm reads the arguments the
    // arms use, and the method is the user's, whose own code then leaves
    // them unused, beside the sentence or not.
    if cell.cfg == Cfg::EveryOff {
        return Some(Verdict::Unmodelled(
            "arms of the method's own over an enum empty in this build",
        ));
    }
    None
}

/// Returns whether a cell whose subject holds a trait object is canonical.
/// A `deref` variant holding a trait object is "how an enum holds an open
/// set beside its sized payloads" (routes.md), so the second variant holds
/// it, which way round the two stand being no matter; and the trait is
/// dyn-compatible beside it, so what would make it not, a `Self` in a
/// signature, an `impl Trait` return, a generic or `async` method, an
/// associated item, a generic trait or a second trait, is the plain
/// trait's, as are the conversions, which name the box, an inherent method,
/// which calls a payload's own method that a trait object does not have,
/// and another crate's enum or trait.
fn object_canonical(cell: &Cell) -> bool {
    if cell.s0 == Target::DerefObject {
        return false;
    }
    if !cell.holds_object() {
        return true;
    }
    let node = cell.ret.node();
    let opaque = matches!(node.future(), Some((FutureKind::Opaque, _))) || node == Node::Iterator;
    let signature = !node.holds_this()
        && node.associated().is_empty()
        && !opaque
        && cell.arg == Argument::Absent
        && matches!(
            cell.qual,
            Qualifier::Plain | Qualifier::Unsafe | Qualifier::Provided
        );
    let trait_alone = cell.generic == Generics::Bare
        && cell.agree == Agreement::Absent
        && cell.stack == Stack::Absent
        && cell.conv == Conversion::Unchecked;
    let place = cell.path_descriptor().site != PathSite::Inherent
        && cell.decl != Declaration::Foreign
        && cell.owner == Owner::Local;
    signature && trait_alone && place
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
    if node.borrows_from_the_call() && lends == Lends::Nothing && !borrowed_argument {
        return Some("a reference return under a receiver that holds no lifetime");
    }
    // A boxed future is spelled as `#[async_trait]` writes it, its lifetime
    // named and bound by what the method takes, so it is Rust under every
    // receiver.
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
        && cell.callee == Callee::Method
        && cell.lift == Lift::Bare
        && cell.item == Item::Bare
        && cell.home == Home::Local
        && cell.generic == Generics::Bare
        && cell.conv == Conversion::Unchecked
        && cell.agree == Agreement::Absent
        && cell.stack == Stack::Absent
        && cell.deprecated == Deprecation::Absent
        && cell.s2 == Third::Absent
        && cell.decl == Declaration::Dispatchable
        && cell.owner == Owner::Local
}

/// The subjects and blocks the macro cannot read, or `None` where the
/// cell is judged as any other. "The marked field of a variant takes
/// `deref` and `via = route` only: `from` and `try_from` go on the
/// variant, and a trait named on the field [..] is reported", and "a macro
/// invocation in a partial impl block is not read [..]. It is reported"
/// (limits.md). Either is one sentence whatever the method, so only the
/// axes that make it so and the variants beside it depart from the
/// neutral cell; a cell with both is non-canonical.
///
/// Open question: the documents do not say whether a sentence about the
/// other variants' shapes, a unit variant say, is reported beside one that
/// stops the subject or the block from being read; the macro's `tests/ui`
/// cases hold the sentence alone, and so does the model.
fn unread(cell: &Cell) -> Option<Verdict> {
    let refused = cell.s0.descriptor().refused;
    let invocation = cell.path.descriptor().invocation;
    if !refused && !invocation {
        return None;
    }
    let neutral = Cell {
        s0: cell.s0,
        s1: cell.s1,
        s2: cell.s2,
        path: cell.path,
        ..Cell::neutral()
    };
    let path_departs = cell.path != super::Path::Attribute && !invocation;
    if *cell != neutral || (refused && invocation) || path_departs {
        return Some(Verdict::NonCanonical);
    }
    let (limit, anchor) = match refused {
        true => (Limit::VariantFieldOption, Anchor::Variant(0)),
        false => (Limit::MacroInBlock, Anchor::BlockInvocation),
    };
    Some(Verdict::Limit(vec![Expected { limit, anchor }]))
}

/// The shapes a restated enum takes (restatements.md, "Enums from other
/// crates"), or `None` where the cell is judged as any other. The real
/// enum is another crate's, generic over what its variants hold, as the
/// golden `restated_enum` and the behaviour test `restated_enum` write it.
fn owned(cell: &Cell) -> Option<Verdict> {
    if cell.owner == Owner::Local {
        return None;
    }
    if cell.s0.descriptor().is_struct() {
        return Some(Verdict::NonCanonical);
    }
    let first = matches!(cell.s0, Target::Plain | Target::DerefBox);
    let second = matches!(
        cell.s1,
        Second::Plain | Second::Absent | Second::DerefBox | Second::Unit
    );
    let third = matches!(
        cell.s2,
        Third::Absent | Third::Plain | Third::DerefBox | Third::Unit
    );
    if !(first && second && third) {
        return Some(Verdict::Unmodelled(
            "a restated enum over a variant other than a payload, a boxed payload or a unit",
        ));
    }
    if cell.home == Home::Crate || cell.decl == Declaration::Foreign {
        return Some(Verdict::Unmodelled(
            "a restated enum beside a trait of another crate, an impl the orphan rules refuse",
        ));
    }
    if cell.path.descriptor().site == PathSite::Inherent {
        return Some(Verdict::Unmodelled(
            "an inherent block over a restated enum, which is its sentence",
        ));
    }
    let plain = cell.generic == Generics::Bare
        && cell.cfg == Cfg::Absent
        && cell.deprecated == Deprecation::Absent;
    if !plain {
        return Some(Verdict::Unmodelled(
            "a restated enum with generics, `#[cfg]` or `#[deprecated]` the real enum would share",
        ));
    }
    // The enum's own conversion options are the sentence; the checked
    // default and a variant's options would ask the law for conversions
    // the restatement does not get.
    let conversion = cell.conv.descriptor();
    if conversion.checked && conversion.option.is_none() {
        return Some(Verdict::Unmodelled(
            "the conversions a restated enum does not get, which no impl beside it can prove",
        ));
    }
    // The payloads are type parameters, so a pinned receiver needs each to
    // be `Unpin`, and an associated type taken from the first needs every
    // other bound to agree, bounds the model spells for no generic subject
    // (as `unpredicted_generics` says of a type-parameter payload).
    if matches!(cell.recv, Receiver::PinShared | Receiver::PinExclusive) {
        return Some(Verdict::Unmodelled(
            "a restated enum under a pinned receiver, which needs its parameters `Unpin`",
        ));
    }
    let typed = cell
        .agree
        .descriptor()
        .items
        .iter()
        .any(|item| item.kind == ItemKind::Type);
    if typed || !cell.ret.node().associated().is_empty() {
        return Some(Verdict::Unmodelled(
            "an associated type over a restated enum, which its parameters would have to agree on",
        ));
    }
    None
}

/// The shapes a trait restated beside the subject or shipped in
/// `whichever::foreign` takes (restatements.md), or `None` where the cell
/// is judged as any other.
fn declared(cell: &Cell) -> Option<Verdict> {
    let declaration = cell.decl.descriptor();
    let path = cell.path.descriptor();
    if declaration.foreign {
        // `whichever::foreign::Display` has one method, `fmt(&self, f)`,
        // which the payloads implement for `core::fmt::Display`; the axes
        // of the method under test, of the trait's own items and of where
        // it lives are `Probe`'s, and a mark of `default` meets a method
        // with no default body in a trait of another crate.
        let neutral = cell.recv == Receiver::Shared
            && cell.ret == Return::Tag
            && cell.arg == Argument::Absent
            && cell.qual == Qualifier::Plain
            && cell.item == Item::Bare
            && cell.home == Home::Local
            && cell.generic == Generics::Bare
            && cell.agree == Agreement::Absent
            && cell.stack == Stack::Absent;
        let block = matches!(path.form, PathForm::Default) || path.site == PathSite::Inherent;
        if !neutral || block {
            return Some(Verdict::NonCanonical);
        }
        return None;
    }
    if !declaration.restated {
        return None;
    }
    // An inherent method is no trait's, so the restatement would list the
    // control method alone, which every other cell forwards already.
    if path.site == PathSite::Inherent {
        return Some(Verdict::NonCanonical);
    }
    // The bound is the signature's, which the restatement repeats.
    if !matches!(cell.item, Item::Bare | Item::Sized) {
        return Some(Verdict::Unmodelled(
            "attributes on a restated method, whose home is the restatement's",
        ));
    }
    // Open question: whether a restatement repeats the supertrait the real
    // trait requires is not documented.
    if cell.stack == Stack::Required {
        return Some(Verdict::Unmodelled(
            "a restated trait that requires a supertrait",
        ));
    }
    if declaration.lists_method {
        return None;
    }
    // A restatement that leaves the method out forwards it not at all, so
    // the impl takes the real trait's default; a required method has none,
    // and a mark or a mismatch would name a method the restatement does
    // not have.
    if path.in_block || cell.arg != Argument::Absent {
        return Some(Verdict::Unmodelled(
            "a mark or a mismatch for a method the restatement leaves out",
        ));
    }
    None
}

/// The model cannot say what the macro answers, or cannot write the
/// payloads.
fn unpredicted(cell: &Cell) -> Option<&'static str> {
    let node = cell.ret.node();
    let receiver = cell.recv.descriptor();
    let argument = cell.arg.descriptor();
    let qualifier = cell.qual.descriptor();

    if node.borrows_from_the_call() && receiver.lends == Lends::Nothing {
        return Some("a reference return that borrows from the `Self` argument");
    }
    // A return for the subject's own lifetime names `'a`, which only a
    // subject over `&'a P0` declares, and only an inherent method can
    // spell; the model writes it for a place read through such variants.
    if cell.ret == Return::Lent {
        let inherent = cell.path_descriptor().site == PathSite::Inherent;
        let place = matches!(cell.callee, Callee::Value | Callee::Field);
        let lent = cell
            .live_slots()
            .iter()
            .all(|(_, target, _)| *target == Target::DerefShared);
        let plain = cell.lift == Lift::Bare
            && cell.arg == Argument::Absent
            && cell.cfg == Cfg::Absent
            && cell.qual == Qualifier::Plain;
        if !(inherent && place && lent && plain) {
            return Some(
                "a return for the subject's lifetime outside an inherent place over `&'a P0`",
            );
        }
    }
    // rustc warns of `#[inline]` on a method with no body in the trait
    // itself, and is phasing it out, so the attribute is the provided
    // method's.
    if cell.item == Item::Inline && !qualifier.provided {
        return Some("`#[inline]` on a required method, which rustc warns of in the trait");
    }
    let path = cell.path_descriptor();
    if let Some(reason) = unpredicted_generics(cell) {
        return Some(reason);
    }
    if let Some(reason) = unpredicted_conversions(cell) {
        return Some(reason);
    }
    if let Some(reason) = unpredicted_stack(cell) {
        return Some(reason);
    }
    // Where the first arm of the method returning the trait's own type is
    // not of the type the enum takes, the supplier's, rustc reports the
    // arm as `mismatched types` rather than the arms as incompatible, an
    // E0308 the catalogue cannot tell from any other.
    let kinds = returned_kinds(cell);
    let supplied = cell.supplier().and_then(|supplier| {
        cell.agree
            .descriptor()
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Type)
            .map(|item| item.values[supplier.holds])
    });
    if let (Some(first), Some(supplied)) = (kinds.first(), supplied) {
        if *first != supplied {
            return Some(
                "a returned associated type whose first arm is not the supplier's, a bare E0308",
            );
        }
    }
    // An enum whose every variant is under `#[cfg]` "is matched as `match
    // *self` with `ref` bindings where the receiver is a reference, so a
    // build that leaves no variant still compiles, as a hand's does"
    // (what-is-generated.md). In that build there is no value to build, so
    // the law has nothing to run, and the cell passes when it compiles
    // without a diagnostic. The hand's impl over the empty enum carries the
    // `#[allow(unused_variables)]` a hand needs for the arguments no arm
    // reads; the subject's impl is `#[automatically_derived]` and is not
    // asked the same, and a partial block's or an inherent block's method
    // allows them itself, as a hand's does (CHANGELOG, Unreleased).
    //
    // A method with a `Self` argument keeps its tuple scrutinee, "which has
    // the fallback arm in every build" (docs/decisions/an-enum-that-may-be-
    // empty-is-matched-by-place.md), and a mark may name a variant under
    // `#[cfg]`: "where it is compiled out, the method takes its body"
    // (inherent-methods.md; partial-impl-blocks.md: a block's method names
    // its variants "as an inherent method does").
    let inherent = path.site == PathSite::Inherent;
    if cell.s0.descriptor().is_struct() {
        // "Two values of the struct always hold the same field, and the
        // argument is unwrapped to it" (structs.md): no mismatch to give.
        if argument.kind.is_some() && argument.mismatch == MismatchSite::Enum && !inherent {
            return Some("a mismatch option on a struct, which has no second variant");
        }
        // "A block on a dispatch struct forwards the same way: `*` is the
        // field marked bare, or the only field, and a field's name is that
        // field" (inherent-methods.md); the model names no field.
        if path.form == PathForm::First {
            return Some("a struct's inherent or block method naming its field, not modelled yet");
        }
    }
    if inherent {
        // An inherent method forwards by method-call syntax to the
        // payloads' own `m` (inherent-methods.md), under the receivers the
        // trait's forwarding takes; one that would need a refcount or an
        // allocation is not described for an inherent method.
        let forwarded = match cell.recv {
            Receiver::Shared
            | Receiver::Exclusive
            | Receiver::Owned
            | Receiver::Boxed
            | Receiver::PinShared
            | Receiver::PinExclusive => true,
            Receiver::Rc | Receiver::Arc | Receiver::PinBoxed => false,
        };
        if !forwarded {
            return Some(
                "an inherent method under a receiver that would need a refcount or an allocation",
            );
        }
        if let Some(reason) = unpredicted_callee(cell) {
            return Some(reason);
        }
        // The payload takes the enum it is handed as a type parameter of
        // its own, which a turbofish naming the method's would have to
        // spell too.
        let enum_argument = argument.kind.is_some() && !cell.s0.descriptor().is_struct();
        if enum_argument && cell.qual == Qualifier::Turbofish {
            return Some("an inherent turbofish beside a `Self` argument the payload takes as a type parameter");
        }
        // An inherent opaque future names the receiver's lifetime alone,
        // `+ '_` (golden `async_returns`), so the `async move` block that
        // awaits the arms cannot capture a borrowed argument or a type
        // parameter, which a hand would have to name with `use<..>`.
        let opaque = matches!(node.future(), Some((FutureKind::Opaque, _)));
        let borrowed = matches!(
            argument.kind,
            Some(ArgumentKind::Shared | ArgumentKind::Exclusive)
        );
        let generic = qualifier.generic == Some(Generic::Fixed);
        if opaque && (borrowed || generic) {
            return Some(
                "an inherent opaque future that captures an argument its `+ '_` does not name",
            );
        }
        let supported = match cell.ret {
            Return::Tag
            | Return::Unit
            | Return::Shared
            | Return::Exclusive
            | Return::This
            | Return::OptionThis
            | Return::ResultThis
            | Return::FallibleThis
            | Return::Future
            | Return::Lent => true,
            Return::TupleThis
            | Return::Assoc
            | Return::Out
            | Return::OptionOut
            | Return::ResultError
            | Return::ResultThisError
            | Return::FutureThis
            | Return::BoxedFuture
            | Return::BoxedFutureOut
            | Return::VecThis
            | Return::OptionTupleThis
            | Return::Gat
            | Return::GatUndeclared => false,
            // "A block on a dispatch struct forwards the same way [..]; a
            // return-position `impl Trait` is forwarded as it is"
            // (inherent-methods.md), which an enum's arms cannot.
            Return::Iterator => cell.s0.descriptor().is_struct(),
        };
        if !supported && cell.s0.descriptor().is_struct() {
            return Some("an inherent method with a return type the inherent docs do not describe for a struct");
        }
        if !supported {
            return Some("an inherent method with a return type the inherent docs do not describe for an enum");
        }
        // "A method returning `Self`, `Option<Self>` or `Result<Self, E>`
        // wraps the payload's result back into the variant, as trait
        // dispatch does" (inherent-methods.md), so a variant reached
        // through a route or a marked field gets the trait's sentences,
        // below, ending in the inherent method's way out.
        let method_route = cell
            .method_slots()
            .iter()
            .any(|(_, _, target)| target.ends_in_method());
        if method_route && cell.recv != Receiver::Shared {
            return Some("an inherent method through a route ending in a method call under a receiver other than `&self`");
        }
        let loan_route = cell
            .method_slots()
            .iter()
            .any(|(_, _, target)| target.reads_through_a_loan());
        if loan_route && cell.recv != Receiver::Shared {
            return Some("an inherent method through a field of what a method lends under a receiver other than `&self`");
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
    // every variant, is a constant of the return type, which under a
    // function lift is the lift's own.
    if matches!(
        path.form,
        PathForm::First | PathForm::Written | PathForm::Arms
    ) {
        let node = arm_node(cell);
        if node.holds_future() {
            return Some("a block body for a future return");
        }
        // A restated enum's variant around a fresh payload is of another
        // instantiation than `Self`.
        let nameable = cell.s0.descriptor().storage == Storage::Payload
            && cell.cfg.of(0) != Some("any()")
            && cell.owner == Owner::Local;
        if !node.has_constant(nameable) {
            return Some("a block body for a return type that needs a payload to make");
        }
    }
    if argument.kind.is_some() && argument.mismatch != MismatchSite::Missing && !inherent {
        // "For a method that returns a future, the expression is the
        // future's output where the arms are awaited, in an `async fn`, an
        // `impl Future` and a boxed future whose output is converted, and a
        // value of the declared future where a boxed future is forwarded as
        // it is" (options.md).
        let output = match node.future() {
            Some((_, output)) => *output,
            None => node,
        };
        // A `Self` in the mismatch is the first variant built around a
        // fresh payload, which only the enum's attribute can name: "An
        // expression on the trait method or on the enum may use the
        // method's parameters, a `Self` argument included" (options.md),
        // and the trait names no variant, nor can it make a `Self` of a
        // reference to one. A place no payload holds is leaked.
        let nameable = argument.mismatch == MismatchSite::Enum
            && cell.cfg.of(0) != Some("any()")
            && cell.generic != Generics::Payload
            && cell.owner == Owner::Local;
        let on_trait = argument.mismatch == MismatchSite::Trait;
        if on_trait && output.holds_this() && !output.has_constant(nameable) {
            return Some(
                "a mismatch on the trait answering `Self`, which the trait names no variant of",
            );
        }
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
    // A default body may answer `self` where the method takes it by value
    // or in a box; left to the default by a block, such an answer is the
    // enum itself, which the law has no twin of, since `Defaulted` answers
    // a `Defaulted`.
    let owns = matches!(cell.recv, Receiver::Owned | Receiver::Boxed);
    if qualifier.provided && !node.has_default(owns) {
        return Some("a default body that would need a payload to make its answer");
    }
    if qualifier.provided && path.form == PathForm::Default && node.holds_this() {
        return Some("a default answering `Self` left to the default, whose twin is no enum");
    }
    None
}

/// The second trait and the deprecated variant the model writes.
fn unpredicted_stack(cell: &Cell) -> Option<&'static str> {
    let stack = cell.stack.descriptor();
    // Open question: which of two attributes writes the conversions, and
    // whose `from` decides them, is not documented; the model leaves the
    // default to both, which writes them once (behaviour test
    // `foreign_traits`, `ReadError`).
    if stack.present && cell.conv != Conversion::Unchecked {
        return Some("conversion options beside a second attribute");
    }
    // The type-parameter payload's `from = false` would have to be spelled
    // on the second attribute too.
    if stack.present && cell.generic == Generics::Payload {
        return Some("a type-parameter payload beside a second attribute");
    }
    // The second trait declares the associated items `Probe` does, so a
    // `Self::Kind` in a signature of a trait that requires it finds the
    // name twice, rustc's E0221 in the trait itself.
    if stack.required && cell.agree.descriptor().returned {
        return Some("a returned associated type a required supertrait declares too");
    }
    // A supertrait would name the trait's const parameter, which the second
    // trait does not take.
    if stack.supertrait && cell.generic == Generics::Own {
        return Some("a supertrait generic over the enum's const parameter");
    }
    // The same for a parameter only a block's header declares, and the
    // instantiation's `from = false` would have to be spelled on the
    // second attribute.
    if stack.supertrait && cell.generic == Generics::Every {
        return Some("a supertrait over a trait whose parameter only a block's header declares");
    }
    if stack.present && cell.generic == Generics::Instance {
        return Some("a block for one instantiation beside a second attribute");
    }
    // "The deprecation reaches the code that names the variant"
    // (what-is-generated.md): a mismatch or a body the user writes names
    // the first variant, and would be warned of as the user's own code; so
    // does the constant an arm of the method's own answers for `Self`.
    let path = cell.path_descriptor();
    let names_first = cell.arg.descriptor().kind.is_some()
        || matches!(
            path.form,
            PathForm::First | PathForm::Written | PathForm::Arms
        );
    // An arm of the method's own names the second variant.
    let names_second = path.form == PathForm::Arms;
    if cell.deprecated == Deprecation::Second && names_second {
        return Some("an arm of the user's naming the deprecated variant");
    }
    if cell.deprecated == Deprecation::First && names_first {
        return Some("a mismatch or a body of the user's naming the deprecated variant");
    }
    None
}

/// The conversions the model checks: over payloads it builds by value,
/// in a build that holds every variant.
fn unpredicted_conversions(cell: &Cell) -> Option<&'static str> {
    let conversion = cell.conv.descriptor();
    if !conversion.checked {
        return None;
    }
    let live = cell.live_slots();
    if live.iter().any(|(_, _, target)| target.borrows()) {
        return Some(
            "conversions of a subject that borrows, whose payloads the law cannot hand over",
        );
    }
    if cell.cfg != Cfg::Absent {
        return Some("conversions beside variants under `#[cfg]`");
    }
    let is_struct = cell.s0.descriptor().is_struct();
    if is_struct && conversion.variant_option.is_some() {
        return Some("a variant's conversion option on a struct, which has no variant");
    }
    // `TryFrom` is written for the type a variant of one field stores, as
    // `From` is: the golden `inherent` writes `TryFrom<AnyShape> for
    // Box<Square>` for a `deref` variant over `Box<Square>`, and a variant
    // with other fields gets none (routes.md); see `Cell::try_slots`.
    //
    // A type-parameter payload's conversions are the parameter's own,
    // which the docs bound for the bare parameter alone.
    if cell.generic == Generics::Payload && cell.s0 != Target::Plain {
        return Some("conversions of a type-parameter payload behind a pointer");
    }
    // Open question: `into` converts "each payload"; for a variant that
    // stores a pointer, a wrapper or a pair, whether that is the stored
    // type or the target is not said.
    if conversion.variant_option.is_some() && cell.s0 != Target::Plain {
        return Some("a variant's conversion option beside a variant option of its own");
    }
    if conversion.into && !is_struct {
        let reached = live.iter().all(|(_, target, descriptor)| {
            *target == Target::Plain || (descriptor.has_other_fields() && !descriptor.is_routed())
        });
        let pair = cell
            .slots()
            .iter()
            .any(|slot| slot.variant == super::SlotVariant::Pair);
        if !reached || pair {
            return Some(
                "`into` over a variant that stores more than its payload, or several fields",
            );
        }
    }
    None
}

/// The generics the model writes: one parameter, of the trait or of the
/// subject, on a subject that borrows nothing.
fn unpredicted_generics(cell: &Cell) -> Option<&'static str> {
    if cell.generic == Generics::Bare {
        return None;
    }
    let borrows = cell
        .live_slots()
        .iter()
        .any(|(_, _, target)| target.borrows());
    if borrows {
        return Some("a generic beside the lifetime the subject takes");
    }
    // An associated type taken from a type parameter is the parameter's
    // own, which a hand bounds or overrides; the model spells neither.
    let typed = cell
        .agree
        .descriptor()
        .items
        .iter()
        .any(|item| item.kind == ItemKind::Type);
    if !cell.ret.node().associated().is_empty() || typed {
        return Some(
            "an associated type beside generics, which the model spells for the plain trait",
        );
    }
    // A variant compiled out may leave the subject's parameter unused; a
    // trait's parameter is the trait's, which the subject only names.
    let parameter = cell.generic.descriptor().parameter.is_some();
    if parameter && cell.cfg != Cfg::Absent {
        return Some("a subject's generic parameter beside variants under `#[cfg]`");
    }
    // A generic struct is documented, "`impl<T: Shape> Shape for
    // Wrapper<T>` for `struct Wrapper<T>(T)`" (options.md); the law builds
    // and takes apart a tuple struct by its path with the argument spelled,
    // `E::<P0>(..)`, which an alias cannot.
    if !matches!(cell.generic, Generics::Payload | Generics::Instance) {
        return None;
    }
    // "A payload that is a type parameter of the enum, or `Box<T>` under
    // `deref`, gives the impl the bound `Raw: Trait`" (options.md); a
    // partial block's header is the user's, and spells the bound itself.
    // A wrapper or a pair the model writes holds the payload's own type,
    // which would leave the parameter unused.
    if !matches!(cell.s0, Target::Plain | Target::DerefBox | Target::Struct) {
        return Some("a type-parameter payload reached through a route or a marked field");
    }
    // An impl for one instantiation names the payload's type, which needs
    // no bound and is `Unpin`.
    if cell.generic == Generics::Instance {
        return None;
    }
    if cell.path != super::Path::Attribute {
        return Some(
            "a type-parameter payload through a partial block, whose header is the user's",
        );
    }
    // A pinned receiver needs every payload to be `Unpin`
    // (what-is-generated.md), a bound on the parameter the model does not
    // write.
    if matches!(cell.recv, Receiver::PinShared | Receiver::PinExclusive) {
        return Some("a type-parameter payload under a pinned receiver, which needs `T: Unpin`");
    }
    None
}

/// The callees and lifts the model writes, as inherent-methods.md
/// describes them; the rest is unmodelled.
fn unpredicted_callee(cell: &Cell) -> Option<&'static str> {
    let plain_place = cell
        .method_slots()
        .iter()
        .all(|(_, _, target)| !target.is_routed());
    let lift = cell.lift;
    // An inherent method that names the first variant has a body for the
    // rest, a constant of the declared type; only rendering builds that
    // type, so its constant is read here from the target's.
    let first = matches!(
        cell.path_descriptor().form,
        PathForm::First | PathForm::Arms
    );
    let constant = match lift {
        Lift::Bare | Lift::Into => cell.target_node().has_constant(false),
        Lift::Some | Lift::Ok | Lift::ByRef | Lift::ByValue => true,
    };
    if first && !constant {
        return Some("an inherent body for a return type that needs a payload to make");
    }
    match cell.callee {
        Callee::Method | Callee::Renamed => {
            if let Lifted::Unpredicted(reason) = lifted(cell) {
                return Some(reason);
            }
        }
        Callee::Value | Callee::Field => {
            if !matches!(
                cell.ret,
                Return::Tag | Return::Shared | Return::Exclusive | Return::Lent
            ) {
                return Some("a place read as a return type other than a value or a reference");
            }
            if cell.qual != Qualifier::Plain {
                return Some("a place under an `async`, `unsafe` or generic method");
            }
            // A place reads the target alone, so a `Self` argument would be
            // the user's own unused argument.
            if cell.arg != Argument::Absent {
                return Some("a place beside a `Self` argument, which nothing reads");
            }
            // "Any other lift is handed the target as it stands"
            // (inherent-methods.md), which for `Into::into` is the binding
            // or the place, converted into a type of the method's choosing.
            if lift == Lift::Into {
                return Some("a place lifted through `Into::into`, not modelled yet");
            }
            let function = matches!(lift, Lift::ByRef | Lift::ByValue);
            if function && cell.ret != Return::Tag {
                return Some("a function lift declared with a return type other than its own");
            }
            if function && !plain_place {
                return Some("a function lift handed a place reached through a route");
            }
        }
    }
    None
}

/// How a lift over an inherent method's call meets what the call answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lifted {
    /// The lift takes the answer, or there is no lift.
    Takes,
    /// rustc refuses the lift's call, with the code, for the reason.
    Refused(&'static str, &'static str),
    /// The model does not predict it, for the reason.
    Unpredicted(&'static str),
}

/// Returns how the cell's lift meets what its method's call answers.
/// "`with = path` wraps each forwarded result in a call to `path`"
/// (inherent-methods.md). A method returning `Self` or `Option<Self>`
/// wraps the payload's result back into the variant, and "with `with`,
/// the payload returns the plain value and `with` lifts the wrapped
/// variant"; the arms are awaited where the method is `async` or returns
/// `impl Future`. Under `support::by_ref` and `support::by_value` the
/// declared type is theirs, `u64`, which names neither, so the function
/// is handed the payload's answer as it is, the payload's own type or its
/// future included; the payloads count as `support::Counted` there. Under
/// the other lifts the awaited answer is not modelled yet.
fn lifted(cell: &Cell) -> Lifted {
    let future = cell.ret.node().future().is_some();
    let unpredicted =
        Lifted::Unpredicted("a lift over an answer whose type the model does not spell");
    match cell.lift {
        Lift::Bare => Lifted::Takes,
        Lift::Some | Lift::Ok | Lift::Into if future => Lifted::Unpredicted("a lift over a future"),
        Lift::Some | Lift::Ok => Lifted::Takes,
        // `with = Into::into` "rebuilds the enum through its `From`
        // impls": the payload answers its own type, which converts into
        // the enum named in the return type, or a value that converts
        // into itself.
        Lift::Into => match cell.ret {
            Return::Tag | Return::Unit | Return::Shared | Return::Exclusive => Lifted::Takes,
            Return::This if cell.into_rebuilds() => Lifted::Takes,
            Return::This | Return::OptionThis => Lifted::Refused(
                "E0277",
                "`Into::into` handed an answer no `From` impl converts into the declared type",
            ),
            _ => unpredicted,
        },
        // A function taking `&T` takes a shared reference, and a mutable
        // one coerced to it.
        Lift::ByRef => match cell.ret {
            Return::Shared | Return::Exclusive | Return::Lent => Lifted::Takes,
            Return::Tag | Return::Unit | Return::This | Return::OptionThis | Return::Future => {
                Lifted::Refused(
                    "E0308",
                    "a function taking `&T` handed an answer that is no reference",
                )
            }
            _ => unpredicted,
        },
        // `support::by_value` takes a `T: Counted`, which `u64` and the
        // payloads are.
        Lift::ByValue => match cell.ret {
            Return::Tag | Return::This => Lifted::Takes,
            Return::Unit
            | Return::Shared
            | Return::Exclusive
            | Return::Lent
            | Return::OptionThis
            | Return::Future => Lifted::Refused(
                "E0277",
                "a function taking a `T: Counted` handed an answer that is not counted",
            ),
            _ => unpredicted,
        },
    }
}

/// Returns the return type the arms of the method under test meet. A
/// function lift's declared type is its own, `u64`, which names no `Self`
/// and no future, so its arms rewrap and await nothing and no limit of the
/// payload's answer concerns them (`lifted`).
fn arm_node(cell: &Cell) -> Node {
    match cell.lift {
        Lift::ByRef | Lift::ByValue => cell.declared_node(),
        Lift::Bare | Lift::Some | Lift::Ok | Lift::Into => cell.ret.node(),
    }
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
    let node = arm_node(cell);
    let receiver = cell.recv.descriptor();
    let argument = cell.reached_argument();
    let slots = cell.slots();
    let path = cell.path_descriptor();
    // The variant-specific rules of `m` concern the variants it is
    // forwarded to, and the method-wide ones apply when it is forwarded at
    // all: a method a block writes, or leaves to the default, is no
    // forwarding.
    let method_live = cell.method_slots();
    let forwarded = matches!(
        path.form,
        PathForm::Forwarded | PathForm::First | PathForm::Arms
    );

    // A sentence on the whole method sits on its signature in the block
    // when the block holds it, and on the trait's method otherwise.
    let on_method = match path.in_block {
        true => Anchor::BlockSignature,
        false => Anchor::Method,
    };

    // A method has no body for the rest when its mark names every variant,
    // `*` or "a method that names every variant one by one, since no
    // variant is left to it" (inherent-methods.md,
    // `inherent_list_of_every_variant_with_body`); the model gives a body
    // under `*` no arms, so the sentence it reaches is the named list's.
    // `*` names a variant that holds no payload too
    // (`inherent_all_with_unforwardable`), whose way out depends on whether
    // the method may hold arms of its own (`wording`).
    // "Naming every variant, the conditional one included, is exhaustive in
    // every build, so the method takes no body" (behaviour test
    // `inherent_list_naming_a_conditional_variant`): the mark names `A`,
    // so the variants counted are those the enum declares.
    // Under `*` with arms of its own, "a variant with neither a payload nor
    // an arm is an error on that variant" (the decision record), and an arm
    // under a `#[cfg]` that does not hold leaves its variant forwarded.
    let unreachable = slots
        .iter()
        .any(|slot| slot.target().is_none() && !cell.answered_by_arm(slot));
    if path.form == PathForm::First && cell.declared_slots().len() == 1 {
        expect(Limit::BodyNeverRuns, Anchor::BlockSignature);
    }
    if path.mark == Some("*") && unreachable {
        expect(Limit::StarOverUnforwardable, Anchor::BlockSignature);
    }
    if path.form == PathForm::Arms {
        // "A body under `*` that is anything but one `match self` would
        // never run, and is reported too" (limits.md); "A struct has one
        // field and no variant for an arm to name, so under
        // `#[dispatch(*)]` a body would never run, whatever it holds"
        // (`inherent_all_with_body_on_a_struct`).
        let unmatched = cell.arms == Arms::Unmatched || cell.s0.descriptor().is_struct();
        if unmatched {
            expect(Limit::BodyNeverRuns, Anchor::BlockSignature);
        }
        // Open question: the documents do not order the sentences on the
        // body; a wildcard arm, as a body that is no `match self`, is read
        // as reported alone, before what the arms would meet.
        let wildcard = cell.arms == Arms::Wildcard;
        // "A trait method with a `Self` argument, which is matched beside
        // `self`, holds no arms the enum answers itself under `*`" and "A
        // method whose receiver is a reference [..] over an enum whose
        // every variant is under `#[cfg]` [..] holds no arms the enum
        // answers itself" (limits.md).
        if !unmatched && !wildcard && cell.reached_argument().kind.is_some() {
            expect(Limit::ArmsBesideSelfArgument, Anchor::BlockSignature);
        }
        if !unmatched && !wildcard && cell.matched_by_place() {
            expect(Limit::ArmsMatchedByPlace, Anchor::BlockSignature);
        }
        // "Under `*`, an arm the enum answers itself names its variants by
        // path, `Self::Name(..)`; one that names none, `_` or a binding, is
        // reported" (limits.md).
        if wildcard {
            expect(Limit::ArmNamingNoVariant, Anchor::Arm);
        }
    }

    // "`#[dispatch(default)]` on a bodiless method leaves it to the trait's
    // default" (partial-impl-blocks.md), which a required method has none of.
    // "Over a restatement that is the real trait's default, which the
    // restatement cannot see", so no sentence is written there, and the
    // impl lacks the method as a hand's does.
    let restated = cell.decl.descriptor().restated;
    if path.form == PathForm::Default && !cell.qual.descriptor().provided && !restated {
        expect(Limit::DefaultWithoutDefault, Anchor::BlockMethod);
    }

    // "A method that requires `Self: Sized` cannot be forwarded to a
    // `deref` variant holding a trait object; a partial block leaves it to
    // the trait's default with `#[dispatch(default)]`" (limits.md,
    // `dyn_payload_under_sized_method`). The sentence names the variant, so
    // it is that variant's arm, under its `#[cfg]`: "A sentence about one
    // variant, whether about its shape or about the way a method reaches
    // it, is reported under that variant's `#[cfg]` [..] and each variant a
    // method cannot reach is named in a sentence of its own" (limits.md).
    //
    // Open question: the snapshot holds the enum's attribute alone, the
    // sentence on the trait's method; on a block that holds the method it
    // is read as staying there, on "the offending trait item" (limits.md),
    // the declaration whose bound the trait object does not meet.
    if forwarded && cell.requires_sized() {
        for (_, _, target) in &method_live {
            if target.is_object() {
                expect(Limit::SizedOnObject, Anchor::Method);
            }
        }
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
    // to is how a trait reaches the others, the control method included,
    // so the sentence stands where some method is forwarded to every
    // variant with no mark: the control method, or the method under test
    // a block is silent about.
    let silent = path.form == PathForm::Forwarded && path.mark.is_none() && cell.method_present();
    // The second trait is forwarded to every variant through its own
    // attribute.
    let stacked = cell.stack.descriptor().present;
    let dispatched = cell.base_reaches_all() || silent || stacked;
    for slot in slots.iter().filter(|_| dispatched) {
        match slot.variant {
            super::SlotVariant::Live(_) => {}
            super::SlotVariant::Unit => expect(Limit::UnitVariant, Anchor::Variant(slot.index)),
            super::SlotVariant::Pair => expect(Limit::TwoFields, Anchor::Variant(slot.index)),
        }
    }

    // `mismatch` in docs/options.md: a `Self` argument needs the result
    // for two different variants, which one variant never meets. The
    // variants counted are those the enum declares, since some build holds
    // a variant compiled out in this one.
    if path.form == PathForm::Forwarded
        && argument.kind.is_some()
        && argument.mismatch == MismatchSite::Missing
        && cell.needs_mismatch()
    {
        expect(Limit::SelfArgumentWithoutMismatch, Anchor::Method);
    }

    // "An enum cannot dispatch a return-position `impl Trait`, at any depth
    // of the return type, except `impl Future<Output = T>`."
    // With `with`, the payload returns the plain value and the lift wraps
    // the variant (inherent-methods.md); "a declared return that wraps
    // `Self` twice, `Option<Option<Self>>`, is reported" (limits.md,
    // `lift_over_a_wrapped_self`).
    let lifted = matches!(cell.lift, Lift::Some | Lift::Ok);
    if lifted && node.holds_this() && node != Node::This {
        expect(Limit::SelfInReturn, on_method);
    }

    // "A struct's `match` has one arm, so it forwards a return-position
    // `impl Trait` as it is, which an enum cannot" (structs.md).
    let one_arm = cell.s0.descriptor().is_struct();
    let opaque = forwarded && !one_arm && matches!(node, Node::Iterator);
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
    //
    // Open question: a route of several steps that starts with `*`, `via =
    // *.payload`, is read as a route, whose sentence names it, rather than
    // as `deref`; `tests/ui` pins `deref` alone and a field alone.
    //
    // Under `with = Into::into` the return type names the enum rather than
    // `Self`, so the payload's answer reaches `Into::into` as it is and
    // nothing is rewrapped; a variant without a `From` impl for it is a
    // rejection (`lifted`).
    let rewraps = cell.lift != Lift::Into;
    if rewraps && conversion.is_some() && node.holds_this() {
        for (slot, _, target) in &method_live {
            let anchor = Anchor::Variant(slot.index);
            if target.is_deref() {
                expect(Limit::DerefSelfReturn, anchor);
            } else if target.is_routed() {
                expect(Limit::RouteSelfReturn, anchor);
            } else if target.has_other_fields() {
                expect(Limit::SelfReturnOtherFields, anchor);
            }
        }
    }

    // A struct whose one field is named has no path that builds it, so
    // `Self` inside an `Option` or a `Result`, which `.map(..)` rebuilds
    // through a path, is reported on the field; `-> Self` is rebuilt by
    // the braces (docs/decisions/a-named-field-is-not-rebuilt-under-a-
    // wrapper.md).
    //
    // "The same holds for a variant whose one field is named and marked,
    // `Named { #[dispatch] shape: Square }`: write it as a tuple variant"
    // (limits.md); its sentence names the variant and the tuple variant.
    let mapped = rewraps
        && forwarded
        && node
            .conversion()
            .is_some_and(|conversion| maps(&conversion));
    if mapped && cell.s0.descriptor().form == Form::StructNamed {
        expect(Limit::NamedFieldUnderWrapper, Anchor::Variant(0));
    }
    if mapped {
        for (slot, _, target) in &method_live {
            if target.form == Form::MarkedNamed {
                expect(Limit::NamedVariantUnderWrapper, Anchor::Variant(slot.index));
            }
        }
    }

    // "`self: Box<Self>` on `deref` variants holding a `Box`"; a plain
    // variant is an error, since boxing it again would allocate, and a
    // `deref` variant over another pointer has no `Box` to hand over
    // (`box_self_on_a_shared_pointer`).
    //
    // Open question: a variant reached through a field is read as getting
    // the plain variant's sentence. A route that dereferences a pointer and
    // reads a field of what it points at, `*.payload`, holds a pointer but
    // none of the target, so it gets the sentence of a pointer that is no
    // `Box`: "there is no box to hand over" (limits.md), whatever the
    // pointer is.
    let place = matches!(cell.callee, Callee::Value | Callee::Field);
    if cell.recv == Receiver::Boxed && !place {
        for (slot, _, target) in &method_live {
            let deref = target.is_deref();
            let limit = match target.pointer() {
                Some(Pointer::Box) if deref => continue,
                Some(
                    Pointer::Box
                    | Pointer::Rc
                    | Pointer::Arc
                    | Pointer::Shared
                    | Pointer::Exclusive,
                ) => Limit::BoxSelfOnSharedPointer,
                None => Limit::BoxSelfPlain,
            };
            expect(limit, Anchor::Variant(slot.index));
        }
    }

    // "A value cannot be moved out of a `deref` variant that holds
    // anything but a `Box`: a method taking `self` is reported on the
    // payload."
    //
    // A `Self` argument by value gets the same sentence, naming the
    // argument that would move.
    // An inherent place under `self: Box<Self>` is moved too, unless the
    // return type borrows it: the case
    // `place_moves_through_a_shared_pointer_under_a_boxed_receiver` holds
    // the same fragment.
    // A place the return type borrows moves nothing, under `self` by value
    // as under `self: Box<Self>`: the arm hands over the reference the
    // variant holds.
    let moved_place = place && !node.borrows();
    let moves = match cell.recv {
        Receiver::Owned => !place || moved_place,
        Receiver::Boxed => moved_place,
        Receiver::Shared
        | Receiver::Exclusive
        | Receiver::PinShared
        | Receiver::PinExclusive
        | Receiver::Rc
        | Receiver::Arc
        | Receiver::PinBoxed => false,
    };
    if moves || argument.kind == Some(ArgumentKind::Owned) {
        for (slot, _, target) in &method_live {
            if target.pointer().is_some_and(|pointer| !pointer.owns()) {
                expect(
                    Limit::MovesThroughSharedPointer,
                    Anchor::Variant(slot.index),
                );
            }
        }
    }

    // `deref` dispatches `Rc<T>`, `Arc<T>` and `&T` "like a plain `T`"
    // (docs/options.md), but nothing can be borrowed mutably through them:
    // a method that borrows its target mutably gets one sentence on the
    // payload (docs/decisions/a-borrow-a-shared-pointer-cannot-lend-is-a-
    // sentence.md). An inherent place borrows mutably only where its
    // declared type does.
    let exclusive = (receiver.lends == Lends::Exclusive
        || argument.kind == Some(ArgumentKind::Exclusive))
        && (!place || node.borrows_mutably());
    if exclusive {
        for (slot, _, target) in &method_live {
            if target
                .pointer()
                .is_some_and(|pointer| !pointer.lends_mutably())
            {
                expect(
                    Limit::MutableBorrowThroughSharedPointer,
                    Anchor::Variant(slot.index),
                );
            }
        }
    }

    // A `mismatch` on the enum names a method of every build: one under a
    // `#[cfg]` that does not hold takes its mismatch on the trait method
    // (docs/decisions/an-option-names-an-item-of-every-build.md), and the
    // option is reported where it is written.
    let enum_mismatch = path.form == PathForm::Forwarded
        && path.site != PathSite::Inherent
        && cell.arg.descriptor().kind.is_some()
        && cell.arg.descriptor().mismatch == MismatchSite::Enum;
    if enum_mismatch && !cell.method_present() {
        let anchor = match path.site {
            PathSite::Block => Anchor::BlockAttribute,
            PathSite::Attribute | PathSite::Inherent => Anchor::Attribute,
        };
        expect(Limit::MismatchForAbsentMethod, anchor);
    }

    // "A route ending in a method call cannot serve a `&mut self` method."
    //
    // `self: Pin<&mut Self>` and a `&mut Self` argument get the same
    // sentence, naming the receiver or the argument as the signature has
    // it.
    if receiver.lends == Lends::Exclusive || argument.kind == Some(ArgumentKind::Exclusive) {
        for (slot, _, target) in &method_live {
            if target.ends_in_method() {
                expect(Limit::MethodRouteOnMutSelf, Anchor::Variant(slot.index));
            }
        }
    }

    // "`from`, `try_from` and `into` are options of an enum; a struct gets
    // no conversions" (struct_from_option), and `into` "converts every
    // variant's payload" (options.md), which a variant with other fields or
    // none does not hold alone.
    let conversion = cell.conv.descriptor();
    if one_arm && conversion.option.is_some() {
        expect(Limit::StructConversion, Anchor::Attribute);
    }
    // "A restated enum gets no conversions, which the orphan rules [..]
    // refuse" (restatements.md), and an option asking for one, or for
    // none, is its sentence (`restated_enum_conversions`).
    if cell.owner == Owner::Remote && conversion.option.is_some() {
        expect(Limit::RestatedConversions, Anchor::Attribute);
    }
    // A variant whose payload is the enum's type parameter: "`I` is a type
    // parameter of `Adapter`, so `From<I>` would overlap the other variants'
    // `From` impls" (`from_overlap_type_parameter`), and "`TryFrom<Kind>
    // for T` would implement a foreign trait for a type parameter, which
    // the orphan rules forbid" (`try_from_type_parameter`), each on the
    // variant. The unchecked default writes `from = false` beside another
    // variant, as the golden `generic_payloads` does.
    //
    // Open question: the sentence names "the other variants' `From`
    // impls"; it is read as standing where another variant gets one.
    let parameter = cell.generic == Generics::Payload
        && cell.conv != Conversion::Unchecked
        && slots.first().is_some_and(|slot| slot.index == 0);
    if parameter {
        let from = cell.from_slots();
        let own = from.iter().any(|slot| slot.index == 0);
        let others = from.iter().any(|slot| slot.index != 0);
        if own && others {
            expect(Limit::FromOverlapTypeParameter, Anchor::Variant(0));
        }
        if conversion.try_from.is_some() && !conversion.first_left_out {
            expect(Limit::TryFromTypeParameter, Anchor::Variant(0));
        }
    }
    if conversion.into && !one_arm {
        for slot in &slots {
            match slot.variant {
                super::SlotVariant::Unit => expect(Limit::IntoWithoutPayload, Anchor::Attribute),
                super::SlotVariant::Pair => {}
                super::SlotVariant::Live(target) => {
                    if target.descriptor().has_other_fields() {
                        expect(Limit::IntoOtherFields, Anchor::Attribute);
                    }
                }
            }
        }
    }

    // `type X;` becomes `type X = <FirstVariant as Trait>::X;`
    // (what-is-generated.md), and a variant whose target has no nameable
    // type takes no part (routes.md): with no variant whose target is
    // named, the sentence of `all_variants_conditional` is on the enum.
    // "An associated type or const comes from the first variant without a
    // `#[cfg]`", so only those can supply it.
    // "A generic associated type is declared by the enum, in a partial impl
    // block; it cannot come from the first variant" (limits.md,
    // `generic_associated_type`), and a block that does not declare it gets
    // the same sentence (`gat_undeclared_in_block`), on the trait's
    // declaration of the type, which the enum reads.
    let gat = node
        .associated()
        .iter()
        .any(|(associated, _)| *associated == Associated::Item);
    if gat && !cell.declares_gat() {
        expect(Limit::GenericAssociatedType, Anchor::Associated);
    }

    let items = cell.agree.descriptor().items;
    let taken = node.associated().iter().any(|(_, overridden)| !overridden)
        || items.iter().any(|item| item.overridden.is_none());
    let supplier = cell.supplier();
    if taken && supplier.is_none() {
        expect(Limit::UnsuppliedType, Anchor::Enum);
    }

    // "Every other variant is checked to agree; a disagreement is reported
    // on that variant", for a type and, "for an integer, a float, `bool`,
    // `char` or `&str` const on an enum without a type or const parameter,
    // a lifetime's being no matter", for a const
    // (what-is-generated.md). A variant whose target has no nameable type
    // "takes no part in the agreement checks" (routes.md), nor does one
    // that holds no payload: a const "of a variant reached through a field
    // or a method, is taken from the first variant without a check"
    // (limits.md), so it disagrees silently.
    //
    // Open question: a variant under a `#[cfg]` that holds is read as
    // checked in the builds that hold it, its check carried under the
    // `cfg` as its arm is.
    // "A type that disagrees shows in the arm of a method that returns it"
    // (routes.md): the arms of `kind` hand over each payload's own type, so
    // where two of them differ the `match` is rustc's E0308, on the enum
    // (`associated_type_disagreement`), beside the agreement check's E0271
    // on the variant, or alone where the variant takes no part in the
    // check. A type no variant supplies holds its sentence, which the arms
    // meet as no type of theirs.
    let kinds = returned_kinds(cell);
    if supplier.is_some() && kinds.iter().any(|kind| *kind != kinds[0]) {
        expect(Limit::ArmsDisagree, Anchor::Enum);
    }
    if let Some(supplier) = supplier {
        for item in items.iter().filter(|item| item.overridden.is_none()) {
            let checked = match item.kind {
                ItemKind::Type => true,
                // The second trait declares the same consts, and its impl,
                // which the enum's attribute writes, has none of a block's
                // header parameters, so its check runs where the enum has
                // none of its own.
                ItemKind::Const => {
                    let enum_generic = cell.generic.descriptor().parameter.is_some()
                        || cell.owner == Owner::Remote;
                    let second = cell.stack.descriptor().present && !enum_generic;
                    item.comparable && (!cell.is_generic() || second)
                }
            };
            if !checked {
                continue;
            }
            let limit = match item.kind {
                ItemKind::Type => Limit::TypeDisagreement,
                ItemKind::Const => Limit::ConstDisagreement,
            };
            for slot in &slots {
                let other = slot.index != supplier.index && slot.names_its_target();
                if other && item.differs(supplier.holds, slot.holds) {
                    expect(limit, Anchor::Variant(slot.index));
                }
            }
        }
    }

    expected.sort();
    expected
}

/// Returns the type each arm of the method returning the trait's own type
/// hands over, in the order of the arms: the type of the payload each
/// variant the control method reaches holds; none where the trait has no
/// such method.
fn returned_kinds(cell: &Cell) -> Vec<&'static str> {
    if !cell.agree.descriptor().returned {
        return Vec::new();
    }
    let Some(kind) = cell
        .agree
        .descriptor()
        .items
        .iter()
        .find(|item| item.kind == ItemKind::Type)
    else {
        return Vec::new();
    };
    cell.base_slots()
        .iter()
        .map(|(slot, _, _)| kind.values[slot.holds])
        .collect()
}

/// Returns whether a conversion rebuilds `Self` through a path that
/// `.map(..)` or `.map_err(..)` takes.
fn maps(conversion: &super::returns::Conversion) -> bool {
    use super::returns::Conversion as Step;
    match conversion {
        Step::Keep | Step::Rewrap | Step::Into => false,
        Step::Map(inner) => matches!(**inner, Step::Rewrap) || maps(inner),
        Step::Result(ok, error) => {
            matches!(**ok, Step::Rewrap)
                || matches!(**error, Step::Rewrap)
                || maps(ok)
                || maps(error)
        }
        Step::Tuple(elements) => elements.iter().any(maps),
    }
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
    let argument = cell.reached_argument();
    let qualifier = cell.qual.descriptor();
    let live = cell.method_slots();
    // The method's body is written wherever it is forwarded, even when every
    // arm is a variant's sentence or no variant is left in the build, so
    // what the body itself captures is rejected whatever the variants hold.
    let form = cell.path_descriptor().form;
    let forwarded = cell.method_present()
        && matches!(form, PathForm::Forwarded | PathForm::First | PathForm::Arms);
    let any = |predicate: &dyn Fn(&super::target::TargetDescriptor) -> bool| {
        live.iter().any(|(_, _, target)| predicate(target))
    };

    // A route ending in a method call hands the method's result over as it
    // is (docs/decisions/an-arm-reaches-its-target-through-a-route.md): a
    // method that lends serves the receivers that take a reference, one
    // that yields the target by value serves `self` by value. The other
    // pairing is a type error the hand's call gets as well.
    let owned = cell.recv == Receiver::Owned || argument.kind == Some(ArgumentKind::Owned);
    // The call is qualified by the trait alone where the route ends in a
    // method (routes.md), so under `self` by value its `Self` is inferred
    // from the reference the method lends, which does not implement the
    // trait; a `Self` argument by value beside a reference receiver is the
    // reference where the payload was expected.
    if any(&|target| target.ends_in_loan()) && owned {
        let code = match cell.recv {
            Receiver::Owned => "E0277",
            Receiver::Shared
            | Receiver::Exclusive
            | Receiver::Boxed
            | Receiver::PinShared
            | Receiver::PinExclusive
            | Receiver::Rc
            | Receiver::Arc
            | Receiver::PinBoxed => "E0308",
        };
        reject(
            code,
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
    // An inherent `value` reads the target itself, and a route ending in a
    // method call hands the method's result over as it is (routes.md: "a
    // route ending in a method call yields a value";
    // docs/decisions/an-arm-reaches-its-target-through-a-route.md). A
    // method that lends yields a reference, so the target declared by
    // value, or handed to `Some` or `Ok`, is the reference, a type error a
    // hand handing the result over as it is gets too.
    //
    // Open question: the decision is written for the trait's forwarding;
    // a place read through a route is read as following it, so `value` is
    // never dereferenced out of what the method lends.
    let by_value_place = cell.callee == Callee::Value
        && cell.target_node() == Node::Payload
        && matches!(cell.lift, Lift::Bare | Lift::Some | Lift::Ok);
    if by_value_place && any(&|target| target.ends_in_loan()) {
        reject(
            "E0308",
            "a lent target read by value, which hands the reference over as it is",
            Forwarded::Method,
        );
    }
    // A route that reads a field of what a method lends, `via =
    // .get().payload`, reaches a place behind the shared reference the call
    // returned (routes.md: `.method()` for what a method returns, "in any
    // sequence"), which can be neither borrowed mutably nor moved out of.
    // The sentence for a route through a method concerns one that ends in
    // the call (limits.md), so a hand's call gets rustc's error, and so
    // does the attribute's.
    // A method left to the default of a real trait that gives it none,
    // through a mark of `default` or a restatement that leaves it out, is
    // missing from the impl, a hand's included.
    let defaulted = cell.path_descriptor().form == PathForm::Default;
    if defaulted && cell.decl.descriptor().restated && !qualifier.provided {
        reject(
            "E0046",
            "a method left to the real trait's default, which it does not have",
            Forwarded::Method,
        );
    }
    // A trait object has no size known to the call: moved out of its box
    // for `self` by value, or pinned again for a pinned receiver, which
    // "needs every payload to be `Unpin`" (what-is-generated.md), it is
    // rustc's E0277 for a hand's call too. A method that requires `Self:
    // Sized` has its sentence as that arm instead.
    let unsized_handover = matches!(
        cell.recv,
        Receiver::Owned | Receiver::PinShared | Receiver::PinExclusive
    );
    if any(&|target| target.is_object()) && unsized_handover && !cell.requires_sized() {
        reject(
            "E0277",
            "a trait object moved out of its box or pinned again, which its size or `Unpin` refuses",
            Forwarded::Method,
        );
    }
    let through_loan = any(&|target| target.reads_through_a_loan());
    let exclusive =
        receiver.lends == Lends::Exclusive || argument.kind == Some(ArgumentKind::Exclusive);
    if through_loan && exclusive {
        reject(
            "E0596",
            "a field of what a method lends, borrowed mutably",
            Forwarded::Method,
        );
    }
    if through_loan && owned {
        reject(
            "E0507",
            "a field of what a method lends, moved out of the loan",
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
    // The second trait's method takes the receiver its own variants allow,
    // over every variant.
    let live_all = cell.live_slots();
    let other_some = live_all
        .iter()
        .any(|(_, _, target)| target.ends_in_temporary());
    let other_every = live_all
        .iter()
        .all(|(_, _, target)| target.ends_in_temporary());
    if cell.stack.descriptor().present && other_some && !other_every {
        reject(
            "E0308",
            "a yielded target handed to the second trait's method, which takes `&self`",
            Forwarded::Base,
        );
    }

    // "Any other lift is handed the target as it stands, the reference the
    // pattern binds under `&self` or a place reached through `via` or
    // `field` by value" (inherent-methods.md): a function taking `&T` is
    // handed a value where the binding owns or the place is a field, and
    // one taking `T` is handed a reference where the binding borrows.
    // A build that compiles every variant out has no arm to hand anything
    // over (an inherent `*` over an enum that may be empty).
    let by_reference = cell.recv.descriptor().lends != Lends::Nothing;
    let value = cell.callee == Callee::Value && !live.is_empty();
    let field = cell.callee == Callee::Field && !live.is_empty();
    if cell.lift == Lift::ByRef && ((value && !by_reference) || field) {
        reject(
            "E0308",
            "a function taking `&T` handed the target or a field by value",
            Forwarded::Method,
        );
    }
    if cell.lift == Lift::ByValue && value && by_reference {
        reject(
            "E0277",
            "a function taking `T` handed the reference the pattern binds",
            Forwarded::Method,
        );
    }
    // A lift over a call that does not take what the call answers is a
    // type error the hand's call of the same function gets as well.
    let called = matches!(cell.callee, Callee::Method | Callee::Renamed);
    if let Lifted::Refused(code, reason) = lifted(cell) {
        if called && forwarded && !live.is_empty() {
            reject(code, reason, Forwarded::Method);
        }
    }

    // A boxed future whose output is converted is one `async move` block
    // that holds the receiver and the arguments across the arms' `.await`
    // (docs/decisions/a-boxed-future-is-rewrapped-by-boxing.md), and the
    // future is `+ Send`, as `#[async_trait]` writes it. The block is
    // `Send` only where the enum is `Sync` under a shared receiver, and
    // `Send` under any other; a variant over an `Rc` is neither, so a
    // hand's block is refused too, with rustc's error for a future, which
    // has no code. A boxed future forwarded as it is holds nothing of the
    // enum.
    let converted = node.future().is_some_and(|(kind, output)| {
        kind == FutureKind::Boxed
            && output
                .conversion()
                .is_some_and(|conversion| !conversion.is_keep())
    });
    let unsendable = cell
        .live_slots()
        .iter()
        .any(|(_, _, target)| target.pointer() == Some(Pointer::Rc));
    if forwarded && converted && unsendable {
        reject(
            "future cannot be sent between threads safely",
            "a converted boxed future, `+ Send`, holding an enum with a variant over an `Rc`",
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
