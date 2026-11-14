//! The model: the axes, what each value means, and the verdict of every
//! cell.
//!
//! An axis is declared in [`axes`]; each value describes itself through a
//! descriptor in the module of its kind, which the renderers interpret.
//! [`verdict`] is the limits table, `docs/limits.md` made executable.

pub mod argument;
pub mod axes;
pub mod limits;
pub mod path;
pub mod qualifier;
pub mod receiver;
pub mod returns;
pub mod target;
mod verdict;

pub use axes::{Argument, Callee, Lift, Path, Qualifier, Receiver, Return, Second, Target};
pub use limits::{Anchor, Expected, Limit};
pub use verdict::verdict;

use crate::cell::Cell;
use target::{SecondSlot, TargetDescriptor};

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
    /// and checked only: the hand must fail with the code, the subject must
    /// fail too, and no error of the subject may sit on an attribute.
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
    /// The slot's index: 0 for `A`, 1 for `B`.
    pub index: usize,
    /// What the variant is.
    pub variant: SlotVariant,
}

impl Slot {
    /// Returns the variant's name.
    pub fn name(&self) -> &'static str {
        ["A", "B"][self.index]
    }

    /// Returns the name of the slot's payload type.
    pub fn payload(&self) -> String {
        format!("P{}", self.index)
    }

    /// Returns the name of the slot's wrapper type.
    pub fn wrapper(&self) -> String {
        format!("W{}", self.index)
    }

    /// Returns the target kind and its description of a live slot.
    pub fn target(&self) -> Option<(Target, TargetDescriptor)> {
        match self.variant {
            SlotVariant::Live(target) => Some((target, target.descriptor())),
            SlotVariant::Unit | SlotVariant::Pair => None,
        }
    }
}

impl Cell {
    /// Returns the subject's variants, in order.
    pub fn slots(&self) -> Vec<Slot> {
        let mut slots = vec![Slot {
            index: 0,
            variant: SlotVariant::Live(self.s0),
        }];
        let second = match self.s1.slot() {
            SecondSlot::Absent => return slots,
            SecondSlot::Live(target) => SlotVariant::Live(target),
            SecondSlot::Unit => SlotVariant::Unit,
            SecondSlot::Pair => SlotVariant::Pair,
        };
        slots.push(Slot {
            index: 1,
            variant: second,
        });
        slots
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

    /// Returns the live slots the method under test is forwarded to.
    pub fn method_slots(&self) -> Vec<(Slot, Target, TargetDescriptor)> {
        let form = self.path.descriptor().form;
        self.live_slots()
            .into_iter()
            .filter(|(slot, _, _)| match form {
                path::Form::Forwarded => true,
                path::Form::First => slot.index == 0,
                path::Form::Default | path::Form::Written => false,
            })
            .collect()
    }

    /// Returns whether the control method is forwarded to every variant;
    /// otherwise it names the first, `#[dispatch(A)]` with a body for the
    /// rest, which is how a block reaches a variant holding no payload.
    pub fn base_reaches_all(&self) -> bool {
        let path = self.path.descriptor();
        let first = path.form == path::Form::First && path.site == path::Site::Block;
        let unreachable = self.slots().iter().any(|slot| slot.target().is_none());
        !(first && unreachable)
    }

    /// Returns the live slots the control method is forwarded to.
    pub fn base_slots(&self) -> Vec<(Slot, Target, TargetDescriptor)> {
        let all = self.base_reaches_all();
        self.live_slots()
            .into_iter()
            .filter(|(slot, _, _)| all || slot.index == 0)
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
