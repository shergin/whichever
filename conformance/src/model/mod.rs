//! The model: the axes, what each value means, and the verdict of every
//! cell.
//!
//! An axis is declared in [`axes`]; each value describes itself through a
//! descriptor in the module of its kind, which the renderers interpret.
//! [`verdict`] is the limits table, `docs/limits.md` made executable.

pub mod agreement;
pub mod argument;
pub mod axes;
pub mod conversion;
pub mod declaration;
pub mod generics;
pub mod home;
pub mod item;
pub mod limits;
pub mod path;
pub mod qualifier;
pub mod receiver;
pub mod returns;
pub mod stack;
pub mod target;
mod verdict;
pub mod wording;

pub use axes::{
    Agreement, Argument, Arms, Callee, Cfg, Conversion, Declaration, Deprecation, Generics, Home,
    Item, Lift, Owner, Path, Qualifier, Receiver, Return, Second, Stack, Target, Third,
};
pub use limits::{Anchor, Expected, Limit};
pub use verdict::verdict;

use crate::cell::Cell;
use target::{LaterSlot, TargetDescriptor};

/// What the model says a cell is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Compiles without a diagnostic, and the law holds.
    Pass,
    /// The macro answers with these sentences and nothing else. The
    /// outcome must be a non-empty subset of them; a cell with one is
    /// exact.
    Limit(Vec<Expected>),
    /// Not Rust, with no subject in sight: the trait or a payload's impl is
    /// rejected by rustc. Rendered without subject or hand; must fail to
    /// compile.
    Malformed(&'static str),
    /// The forwarding cannot be written in Rust, a hand gets this rustc
    /// error, and no sentence is documented. Rendered with subject and hand
    /// and checked only: the hand must fail with the code, and the subject
    /// with the same code and with nothing the hand does not get, no
    /// sentence and no error of another code, none of it on an attribute.
    Rejected {
        /// The hand's error: a code such as `E0596`, or the message of an
        /// error that has none.
        code: &'static str,
        /// Why no hand can write it.
        reason: &'static str,
    },
    /// The model cannot write this cell yet; never selected, counted in
    /// the census.
    Unmodelled(&'static str),
    /// A don't-care axis holds a non-neutral value; never selected.
    NonCanonical,
}

impl Verdict {
    /// Returns the verdict's kind, as the census and the report spell it.
    pub fn kind(&self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Limit(_) => "limit",
            Verdict::Malformed(_) => "malformed",
            Verdict::Rejected { .. } => "rejected",
            Verdict::Unmodelled(_) => "unmodelled",
            Verdict::NonCanonical => "non-canonical",
        }
    }

    /// Returns whether a run holds cells of this verdict.
    pub fn is_selectable(&self) -> bool {
        match self {
            Verdict::Pass
            | Verdict::Limit(_)
            | Verdict::Malformed(_)
            | Verdict::Rejected { .. } => true,
            Verdict::Unmodelled(_) | Verdict::NonCanonical => false,
        }
    }

    /// Returns the verdict in one line: the kind, then the sentences with
    /// their anchors or the reason.
    pub fn describe(&self) -> String {
        match self {
            Verdict::Pass => String::from("pass"),
            Verdict::Limit(expected) => {
                let sentences: Vec<String> = expected
                    .iter()
                    .map(|expected| {
                        format!("{}@{}", expected.limit.entry().key, expected.anchor.name())
                    })
                    .collect();
                format!("limit {}", sentences.join(" "))
            }
            Verdict::Malformed(reason) => format!("malformed: {reason}"),
            Verdict::Rejected { code, reason } => format!("rejected {code}: {reason}"),
            Verdict::Unmodelled(reason) => format!("unmodelled: {reason}"),
            Verdict::NonCanonical => String::from("non-canonical"),
        }
    }
}

/// The type parameters a restated enum declares its payloads as, by slot:
/// the real enum is another crate's, generic over what its variants hold.
pub const RESTATED_PARAMETERS: [&str; 3] = ["T0", "T1", "T2"];

/// What a slot's variant is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotVariant {
    /// A variant over the slot's payload with this target kind.
    Live(Target),
    /// A unit variant.
    Unit,
    /// A variant of two unmarked fields.
    Pair,
}

/// A variant of the subject, by position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The slot's index: 0 for `A`, 1 for `B`, 2 for `C`.
    pub index: usize,
    /// What the variant is.
    pub variant: SlotVariant,
    /// The predicate of the `#[cfg]` the variant is under, if any.
    pub cfg: Option<&'static str>,
    /// The slot whose payload type the variant holds: its own, or the
    /// first slot's where two variants hold one type.
    pub holds: usize,
    /// The type parameter the subject declares the payload as, if any.
    pub parameter: Option<&'static str>,
    /// Whether the variant is `#[deprecated]`.
    pub deprecated: bool,
}

impl Slot {
    /// Returns the variant's name.
    pub fn name(&self) -> &'static str {
        ["A", "B", "C"][self.index]
    }

    /// Returns the name of the slot's payload type.
    pub fn payload(&self) -> String {
        format!("P{}", self.holds)
    }

    /// Returns the payload type as the subject declares it: its type
    /// parameter, or the payload's name.
    pub fn declared_payload(&self) -> String {
        match self.parameter {
            Some(parameter) => String::from(parameter),
            None => self.payload(),
        }
    }

    /// Returns whether the slot holds the payload type of another, which
    /// is then declared once, with that slot.
    pub fn holds_another(&self) -> bool {
        self.holds != self.index
    }

    /// Returns the name of the slot's wrapper type.
    pub fn wrapper(&self) -> String {
        format!("W{}", self.index)
    }

    /// Returns whether the slot's target has a type an associated item can
    /// be taken from: a live variant whose route is `deref` or nothing
    /// (routes.md).
    pub fn names_its_target(&self) -> bool {
        self.target()
            .is_some_and(|(_, target)| target.route.iter().all(|step| *step == target::Step::Deref))
    }

    /// Returns the target kind and its description of a live slot.
    pub fn target(&self) -> Option<(Target, TargetDescriptor)> {
        match self.variant {
            SlotVariant::Live(target) => Some((target, target.descriptor())),
            SlotVariant::Unit | SlotVariant::Pair => None,
        }
    }
}

/// An arm a method under `*` writes in its body for a variant the enum
/// answers itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnArm {
    /// The variant the arm names.
    pub slot: Slot,
    /// The predicate of the `#[cfg]` the arm itself is under, if any.
    pub cfg: Option<&'static str>,
}

impl Cell {
    /// Returns the arms the body of a method under `*` holds, in the order
    /// written: one for the second variant, and one for every later variant
    /// that holds no payload unless the arms name the second alone. A
    /// variant under `#[cfg]` keeps its arm, under the variant's `#[cfg]`
    /// as a hand writes it; a body that is no `match self` and a struct's
    /// hold none.
    pub fn own_arms(&self) -> Vec<OwnArm> {
        let path = self.path_descriptor();
        let matched = !matches!(self.arms, Arms::Unmatched);
        if path.form != path::Form::Arms || !matched || self.s0.descriptor().is_struct() {
            return Vec::new();
        }
        self.declared_slots()
            .into_iter()
            .filter(|slot| match slot.index {
                0 => false,
                1 => true,
                _ => slot.target().is_none() && self.arms != Arms::Second,
            })
            .map(|slot| OwnArm {
                slot,
                cfg: match (slot.index, self.arms) {
                    (1, Arms::CfgOn) => Some("all()"),
                    (1, Arms::CfgOff) => Some("any()"),
                    _ => None,
                },
            })
            .collect()
    }

    /// Returns whether an arm of the method's own answers the variant of a
    /// slot in this build: "An arm under `#[cfg]` answers where the `cfg`
    /// holds and its variant is forwarded where it does not"
    /// (partial-impl-blocks.md).
    pub fn answered_by_arm(&self, slot: &Slot) -> bool {
        self.own_arms()
            .iter()
            .any(|arm| arm.slot.index == slot.index && arm.cfg != Some("any()"))
    }

    /// Returns the subject's variants, in order.
    pub fn slots(&self) -> Vec<Slot> {
        self.declared_slots()
            .into_iter()
            .filter(|slot| slot.cfg != Some("any()"))
            .collect()
    }

    /// Returns every variant the subject declares, those compiled out in
    /// this build included.
    pub fn declared_slots(&self) -> Vec<Slot> {
        let mut slots = vec![Slot {
            index: 0,
            variant: SlotVariant::Live(self.s0),
            cfg: self.cfg.of(0),
            holds: 0,
            parameter: match (self.owner, self.generic.descriptor().payload) {
                (Owner::Remote, _) => Some(RESTATED_PARAMETERS[0]),
                (Owner::Local, true) => Some("T"),
                (Owner::Local, false) => None,
            },
            deprecated: self.deprecated.slot() == Some(0),
        }];
        for (index, later) in [(1, self.s1.slot()), (2, self.s2.slot())] {
            let variant = match later {
                LaterSlot::Absent => continue,
                LaterSlot::Live(target) => SlotVariant::Live(target),
                LaterSlot::Unit => SlotVariant::Unit,
                LaterSlot::Pair => SlotVariant::Pair,
            };
            let holds = match (variant, self.shares_payload()) {
                (SlotVariant::Live(_), true) => 0,
                (SlotVariant::Live(_) | SlotVariant::Unit | SlotVariant::Pair, _) => index,
            };
            slots.push(Slot {
                index,
                variant,
                cfg: self.cfg.of(index),
                holds,
                parameter: match (self.owner, variant) {
                    (Owner::Remote, SlotVariant::Live(_)) => Some(RESTATED_PARAMETERS[index]),
                    (Owner::Local, _) | (Owner::Remote, SlotVariant::Unit | SlotVariant::Pair) => {
                        None
                    }
                },
                deprecated: self.deprecated.slot() == Some(index),
            });
        }
        slots
    }

    /// Returns the `Self` argument as the forwarding reaches into it: an
    /// inherent method over an enum hands the enum over as it is
    /// (inherent-methods.md), so no variant's target is reached through it.
    pub fn reached_argument(&self) -> argument::ArgumentDescriptor {
        let argument = self.arg.descriptor();
        let inherent = self.path_descriptor().site == path::Site::Inherent;
        if inherent && !self.s0.descriptor().is_struct() {
            return argument::ArgumentDescriptor {
                kind: None,
                ..argument
            };
        }
        argument
    }

    /// Returns the slots whose stored type the subject's attribute writes
    /// `From` for: the variants of one unnamed field, each type held once,
    /// none on a struct (options.md, routes.md, structs.md).
    pub fn from_slots(&self) -> Vec<Slot> {
        let conversion = self.conv.descriptor();
        if !conversion.from || self.s0.descriptor().is_struct() {
            return Vec::new();
        }
        self.slots()
            .into_iter()
            .filter(|slot| !(conversion.first_left_out && slot.index == 0))
            .filter(|slot| {
                slot.target()
                    .is_some_and(|(_, target)| target.form == target::Form::Single)
            })
            .collect()
    }

    /// Returns the slots whose stored type the subject's attribute writes
    /// `TryFrom<Enum>` for: the variants of one unnamed field, as for
    /// `From`, so a `deref` variant's pointer, as the golden `inherent`
    /// writes `TryFrom<AnyShape> for Box<Square>`; a variant with other
    /// fields gets none (routes.md).
    ///
    /// Open question: the golden holds a pointer alone; a variant reached
    /// through a field or a method of what it stores, a wrapper or a pair,
    /// is read as converting what it stores, the type its `From` takes.
    pub fn try_slots(&self) -> Vec<Slot> {
        let conversion = self.conv.descriptor();
        self.live_slots()
            .into_iter()
            .filter(|(_, _, target)| target.form == target::Form::Single)
            .map(|(slot, _, _)| slot)
            .filter(|slot| match (conversion.try_from, conversion.first_try) {
                (Some(_), _) => !(conversion.first_left_out && slot.index == 0),
                (None, true) => slot.index == 0,
                (None, false) => false,
            })
            .collect()
    }

    /// Returns whether `with = Into::into` rebuilds the enum from what a
    /// payload's method returning `Self` answers: every variant the method
    /// reaches holds its payload as it is, so the enum's attribute writes a
    /// `From` impl for it (options.md); a struct's attribute writes none
    /// (structs.md). The return type names the enum with its generics, a
    /// lifetime included.
    pub fn into_rebuilds(&self) -> bool {
        let converts = self
            .method_slots()
            .iter()
            .all(|(_, target, _)| *target == Target::Plain);
        converts && !self.s0.descriptor().is_struct()
    }

    /// Returns whether the two variants hold one payload type: an inherent
    /// method that answers the target itself, `value`, declares one type
    /// for every arm, so the targets of different variants have to be one
    /// type, reached as each variant reaches it (inherent-methods.md).
    pub fn shares_payload(&self) -> bool {
        self.path_descriptor().site == path::Site::Inherent && self.callee == Callee::Value
    }

    /// Returns how the method under test reaches the subject: the path's
    /// description, except that a restatement which does not list the
    /// method leaves it to the real trait's default, as `#[dispatch(default)]`
    /// in a block does, with no mark (restatements.md: "a restatement
    /// forwards only the methods it lists").
    pub fn path_descriptor(&self) -> path::PathDescriptor {
        let path = self.path.descriptor();
        if self.decl.descriptor().lists_method {
            return path;
        }
        path::PathDescriptor {
            form: path::Form::Default,
            mark: None,
            in_block: false,
            ..path
        }
    }

    /// Returns the name of the method under test, in backticks, as a
    /// sentence names it.
    pub fn method_name(&self) -> &'static str {
        match self.decl.descriptor().foreign {
            true => "`fmt`",
            false => "`m`",
        }
    }

    /// Returns whether a method taking a `Self` argument needs the result
    /// for two different variants, its fallback arm: the enum declares two
    /// variants, so some build holds both, those compiled out in this
    /// build counted, or one under `#[cfg]`, whose pair keeps "the fallback
    /// arm in every build"
    /// (docs/decisions/an-enum-that-may-be-empty-is-matched-by-place.md).
    ///
    /// Open question: an enum of one variant under `#[cfg]` holds no two
    /// variants in any build; the behaviour test `one_variant_without_mismatch`
    /// leaves the mismatch out for "one variant that is always compiled
    /// in", and the build that leaves no variant needs the fallback arm
    /// for the pair to be exhaustive, so the mismatch is read as asked
    /// for.
    pub fn needs_mismatch(&self) -> bool {
        let declared = self.declared_slots();
        declared.len() > 1 || declared.iter().any(|slot| slot.cfg.is_some())
    }

    /// Returns whether the method under test is matched by place, `match
    /// *self` with `ref` bindings: "A method whose receiver is a reference,
    /// `&self` or `&mut self`, plain or pinned, over an enum whose every
    /// variant is under `#[cfg]`" (limits.md). A `self: Box<Self>` receiver
    /// is no reference: the enum is moved out of its box and matched by
    /// value.
    pub fn matched_by_place(&self) -> bool {
        let by_reference = match self.recv {
            Receiver::Shared
            | Receiver::Exclusive
            | Receiver::PinShared
            | Receiver::PinExclusive => true,
            Receiver::Owned
            | Receiver::Boxed
            | Receiver::Rc
            | Receiver::Arc
            | Receiver::PinBoxed => false,
        };
        self.every_under_cfg() && by_reference
    }

    /// Returns whether the method under test may hold, under `*`, a body of
    /// the arms the enum answers itself. "A trait method with a `Self`
    /// argument, which is matched beside `self`, holds no arms the enum
    /// answers itself under `*`; an inherent method hands a `Self` argument
    /// on as it is and is not concerned" (limits.md), which
    /// `reached_argument` says; nor does a method matched by place.
    pub fn takes_own_arms(&self) -> bool {
        self.reached_argument().kind.is_none() && !self.matched_by_place()
    }

    /// Returns whether every variant is under `#[cfg]`, so that the enum
    /// may be empty in some build and is matched by place where the
    /// receiver is a reference (what-is-generated.md).
    pub fn every_under_cfg(&self) -> bool {
        self.declared_slots().iter().all(|slot| slot.cfg.is_some())
    }

    /// Returns the slot an associated item is taken from: the first
    /// variant without a `#[cfg]` (what-is-generated.md) whose target has a
    /// type to take it from, reached by `deref` or not at all (routes.md,
    /// and the sentence of `all_variants_conditional`: "under `#[cfg]` or
    /// reached through a field or a method, so none can supply").
    ///
    /// Open question: the docs say "the first variant without a
    /// `#[cfg]`"; a first variant reached through a field is read as
    /// passed over for the next, since its target's type cannot be named.
    pub fn supplier(&self) -> Option<Slot> {
        self.declared_slots()
            .into_iter()
            .filter(|slot| slot.cfg.is_none())
            .find(Slot::names_its_target)
    }

    /// Returns whether the subject's impl has a type or const parameter, so
    /// that its associated consts are taken without a check: "on an enum with a
    /// type or const parameter [..] taken from the first variant without a
    /// check" (limits.md); the lifetime the `deref` variants over a
    /// reference give it is no matter.
    ///
    /// The check is a free `const` item, which cannot evaluate a const a
    /// type or const parameter decides
    /// (docs/decisions/associated-consts-are-checked-unless-a-type-can-change-them.md):
    /// one a block's own header declares is such a parameter too, and an
    /// impl for one instantiation leaves none.
    ///
    /// Open question: limits.md speaks of "an enum with a type or const
    /// parameter"; the decision's reason, and its refusal of "silence where
    /// a check is possible", are read as deciding by the impl's parameters.
    pub fn is_generic(&self) -> bool {
        let generics = self.generic.descriptor();
        let parameter = generics.parameter.is_some() && !generics.instantiated;
        parameter || generics.header.is_some() || self.owner == Owner::Remote
    }

    /// Returns the slots whose variant forwards to a payload.
    pub fn live_slots(&self) -> Vec<(Slot, Target, TargetDescriptor)> {
        self.slots()
            .into_iter()
            .filter_map(|slot| {
                let (target, descriptor) = slot.target()?;
                Some((slot, target, descriptor))
            })
            .collect()
    }

    /// Returns the live slots the method under test is forwarded to: none
    /// where the method does not exist in this build.
    pub fn method_slots(&self) -> Vec<(Slot, Target, TargetDescriptor)> {
        if !self.method_present() {
            return Vec::new();
        }
        let form = self.path_descriptor().form;
        self.live_slots()
            .into_iter()
            .filter(|(slot, _, _)| match form {
                path::Form::Forwarded => true,
                path::Form::First => slot.index == 0,
                path::Form::Arms => !self.answered_by_arm(slot),
                path::Form::Default | path::Form::Written => false,
            })
            .collect()
    }

    /// Returns whether the control method is forwarded to every variant;
    /// otherwise it names the variants that hold a payload, `#[dispatch(A,
    /// B)]` with a body for the rest, which is how a block reaches a
    /// variant holding no payload (partial-impl-blocks.md: "the method
    /// names the rest"), so that only the method under test meets that
    /// variant. A block always does so where some variant holds no
    /// payload, so that a mark on the method under test is the one thing
    /// that meets the variant.
    pub fn base_reaches_all(&self) -> bool {
        let block = self.path_descriptor().site == path::Site::Block;
        let unreachable = self.slots().iter().any(|slot| slot.target().is_none());
        !(block && unreachable)
    }

    /// Returns the variants the control method's mark names where it is not
    /// forwarded to every variant: every declared variant that holds a
    /// payload, those compiled out in this build included, since a mark
    /// may name a variant under `#[cfg]` (inherent-methods.md).
    pub fn base_named(&self) -> Vec<Slot> {
        self.declared_slots()
            .into_iter()
            .filter(|slot| slot.target().is_some())
            .collect()
    }

    /// Returns whether the method under test exists in this build: the
    /// trait declares it under no `#[cfg]` or under one that holds.
    pub fn method_present(&self) -> bool {
        self.item.descriptor().present
    }

    /// Returns whether the trait declares the method under test `where
    /// Self: Sized`: where the method says so, where `Self` is a type
    /// argument of its return type, or where a body, an `async fn`'s or a
    /// default, holds a `Self` by value.
    pub fn requires_sized(&self) -> bool {
        let node = self.ret.node();
        if self.item.descriptor().sized || node.holds_this_below() {
            return true;
        }
        let qualifier = self.qual.descriptor();
        let by_value = self.recv == Receiver::Owned
            || self.arg.descriptor().kind == Some(argument::ArgumentKind::Owned);
        if qualifier.asyncness && (by_value || node.holds_this()) {
            return true;
        }
        qualifier.provided && (by_value || node.holds_this())
    }

    /// Returns whether the subject declares the trait's generic associated
    /// type: in a partial block, the one place it can
    /// (partial-impl-blocks.md), where the cell writes the declaration.
    pub fn declares_gat(&self) -> bool {
        self.ret == Return::Gat && self.path_descriptor().site == path::Site::Block
    }

    /// Returns whether the subject declares a variant that holds its
    /// payload as a trait object, compiled out in this build or not.
    pub fn holds_object(&self) -> bool {
        self.declared_slots()
            .iter()
            .any(|slot| slot.target().is_some_and(|(_, target)| target.is_object()))
    }

    /// Returns the live slots the control method is forwarded to.
    pub fn base_slots(&self) -> Vec<(Slot, Target, TargetDescriptor)> {
        let all = self.base_reaches_all();
        let named = self.base_named();
        self.live_slots()
            .into_iter()
            .filter(|(slot, _, _)| all || named.iter().any(|other| other.index == slot.index))
            .collect()
    }

    /// Returns the type an inherent arm's target answers before any lift:
    /// the payload itself, by value or borrowed, for `value`, and the
    /// return type otherwise.
    pub fn target_node(&self) -> returns::Node {
        let node = self.ret.node();
        if self.callee != Callee::Value {
            return node;
        }
        match node {
            returns::Node::Tag => returns::Node::Payload,
            returns::Node::Shared => returns::Node::PayloadShared,
            returns::Node::Lent => returns::Node::PayloadLent,
            returns::Node::Exclusive => returns::Node::PayloadExclusive,
            other => other,
        }
    }

    /// Returns the return type the subject's method declares: the target's
    /// type with the lift applied.
    pub fn declared_node(&self) -> returns::Node {
        let target = self.target_node();
        let leaked = |node: returns::Node| -> &'static returns::Node { Box::leak(Box::new(node)) };
        match self.lift {
            Lift::Bare | Lift::Into => target,
            Lift::Some => returns::Node::Option(leaked(target)),
            Lift::Ok => returns::Node::Result(leaked(target), &returns::Node::Fault),
            Lift::ByRef | Lift::ByValue => returns::Node::Tag,
        }
    }

    /// Returns the cell's verdict.
    pub fn verdict(&self) -> Verdict {
        verdict(self)
    }
}
