//! What a `Self` argument is: its type, and where its mismatch is spelled.

use super::axes::Argument;

/// How the method takes the other value of `Self`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentKind {
    /// `other: &Self`.
    Shared,
    /// `other: &mut Self`.
    Exclusive,
    /// `other: Self`.
    Owned,
}

impl ArgumentKind {
    /// Returns the argument's type.
    pub fn spelling(self) -> &'static str {
        match self {
            ArgumentKind::Shared => "&Self",
            ArgumentKind::Exclusive => "&mut Self",
            ArgumentKind::Owned => "Self",
        }
    }
}

/// Where the result for two different variants is spelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MismatchSite {
    /// `#[dispatch(mismatch = ..)]` on the trait method.
    Trait,
    /// `mismatch(m = ..)` on the enum's attribute.
    Enum,
    /// Nowhere.
    Missing,
}

/// A `Self` argument's description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArgumentDescriptor {
    /// How the argument is taken; `None` when the method takes none.
    pub kind: Option<ArgumentKind>,
    /// Where the mismatch is spelled.
    pub mismatch: MismatchSite,
}

impl Argument {
    /// Returns the argument's description.
    pub fn descriptor(self) -> ArgumentDescriptor {
        let with = |kind, mismatch| ArgumentDescriptor {
            kind: Some(kind),
            mismatch,
        };
        match self {
            Argument::Absent => ArgumentDescriptor {
                kind: None,
                mismatch: MismatchSite::Missing,
            },
            Argument::SharedOnTrait => with(ArgumentKind::Shared, MismatchSite::Trait),
            Argument::SharedOnEnum => with(ArgumentKind::Shared, MismatchSite::Enum),
            Argument::Owned => with(ArgumentKind::Owned, MismatchSite::Enum),
            Argument::Exclusive => with(ArgumentKind::Exclusive, MismatchSite::Trait),
            Argument::SharedBare => with(ArgumentKind::Shared, MismatchSite::Missing),
        }
    }
}
