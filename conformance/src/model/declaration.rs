//! How the trait under test is declared: with `#[whichever::dispatchable]`,
//! restated beside the subject with `remote`, or shipped in
//! `whichever::foreign` (restatements.md; golden `foreign`; behaviour tests
//! `foreign_traits`, `partial_block_over_a_foreign_trait` and
//! `pinned_writers_with_provided_methods`).

use super::axes::Declaration;

/// The module of the cell a restated trait is declared in when it lives in
/// the cell beside the subject: "another module", as the behaviour test
/// `pinned_writers_with_provided_methods` declares its `io`.
pub const REAL: &str = "real";

/// A restatement `whichever::foreign` ships that a cell dispatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foreign {
    /// `whichever::foreign::Display`, whose one method is `fmt`.
    Display,
    /// `whichever::foreign::Iterator`, of whose methods the cell forwards
    /// `next` and the provided `any`, which every payload overrides.
    Iterator,
}

impl Foreign {
    /// Returns the path the restatement is dispatched by.
    pub fn path(self) -> &'static str {
        match self {
            Foreign::Display => "whichever::foreign::Display",
            Foreign::Iterator => "whichever::foreign::Iterator",
        }
    }

    /// Returns the trait the restatement implements, as the hand names it.
    pub fn real(self) -> &'static str {
        match self {
            Foreign::Display => "core::fmt::Display",
            Foreign::Iterator => "core::iter::Iterator",
        }
    }

    /// Returns the last segment of the real trait's path, which an impl of
    /// it is found by in the expansion.
    pub fn name(self) -> &'static str {
        match self {
            Foreign::Display => "Display",
            Foreign::Iterator => "Iterator",
        }
    }

    /// Returns the name of the method under test, in backticks, as a
    /// sentence names it.
    pub fn method_name(self) -> &'static str {
        match self {
            Foreign::Display => "`fmt`",
            Foreign::Iterator => "`any`",
        }
    }
}

/// The declaration's description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclarationDescriptor {
    /// Whether the trait is declared without the attribute and restated
    /// beside the subject with `#[whichever::dispatchable(remote = ..)]`.
    pub restated: bool,
    /// Whether the restatement lists the method under test; one that does
    /// not leaves it to the real trait's default.
    pub lists_method: bool,
    /// The restatement `whichever::foreign` ships that the trait is, if
    /// it is one.
    pub foreign: Option<Foreign>,
    /// Whether the trait is an `unsafe trait`, whose impls are `unsafe
    /// impl`.
    pub unsafety: bool,
}

impl Declaration {
    /// Returns the declaration's description.
    pub fn descriptor(self) -> DeclarationDescriptor {
        let dispatchable = DeclarationDescriptor {
            restated: false,
            lists_method: true,
            foreign: None,
            unsafety: false,
        };
        match self {
            Declaration::Dispatchable => dispatchable,
            Declaration::Remote => DeclarationDescriptor {
                restated: true,
                ..dispatchable
            },
            Declaration::RemoteBase => DeclarationDescriptor {
                restated: true,
                lists_method: false,
                ..dispatchable
            },
            Declaration::Foreign => DeclarationDescriptor {
                foreign: Some(Foreign::Display),
                ..dispatchable
            },
            Declaration::ForeignIterator => DeclarationDescriptor {
                foreign: Some(Foreign::Iterator),
                ..dispatchable
            },
            Declaration::Unsafe => DeclarationDescriptor {
                unsafety: true,
                ..dispatchable
            },
        }
    }
}
