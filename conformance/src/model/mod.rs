//! The model: the axes, what each value means, and the verdict of every
//! cell.
//!
//! An axis is declared in [`axes`]; each value describes itself through a
//! descriptor in the module of its kind, which the renderers interpret.
//! [`verdict`] is the limits table, `docs/limits.md` made executable.

pub mod argument;
pub mod axes;
pub mod limits;
pub mod qualifier;
pub mod receiver;
pub mod returns;
pub mod target;
mod verdict;

pub use axes::{Argument, Qualifier, Receiver, Return, Second, Target};
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
            Verdict::Unmodelled(_) => "unmodelled",
            Verdict::NonCanonical => "non-canonical",
        }
    }

    /// Returns whether a run holds cells of this verdict.
    pub fn is_selectable(&self) -> bool {
        match self {
            Verdict::Pass | Verdict::Limit(_) | Verdict::Malformed(_) => true,
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

    /// Returns the cell's verdict.
    pub fn verdict(&self) -> Verdict {
        verdict(self)
    }
}
