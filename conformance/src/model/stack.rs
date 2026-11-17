//! What the second trait is: a trait dispatched on the same subject by an
//! attribute of its own (options.md: "Repeat the attribute, one trait per
//! attribute. A supertrait is one more attribute"), and what a deprecated
//! variant asks of the arms (what-is-generated.md).

use super::axes::{Deprecation, Stack};

/// The name of the second trait.
pub const OTHER: &str = "Other";

/// The second trait's description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackDescriptor {
    /// Whether a second trait is dispatched on the subject.
    pub present: bool,
    /// The supertrait the second trait declares, if any.
    pub supertrait: bool,
    /// Whether the trait under test requires the second trait as its
    /// supertrait.
    pub required: bool,
}

impl Stack {
    /// Returns the second trait's description.
    pub fn descriptor(self) -> StackDescriptor {
        match self {
            Stack::Absent => StackDescriptor {
                present: false,
                supertrait: false,
                required: false,
            },
            Stack::Second => StackDescriptor {
                present: true,
                supertrait: false,
                required: false,
            },
            Stack::Supertrait => StackDescriptor {
                present: true,
                supertrait: true,
                required: false,
            },
            Stack::Required => StackDescriptor {
                present: true,
                supertrait: false,
                required: true,
            },
        }
    }
}

impl Deprecation {
    /// Returns the slot whose variant is `#[deprecated]`, if any.
    pub fn slot(self) -> Option<usize> {
        match self {
            Deprecation::Absent => None,
            Deprecation::First => Some(0),
            Deprecation::Second => Some(1),
        }
    }
}
