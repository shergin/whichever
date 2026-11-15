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

/// The path `whichever::foreign`'s `Display` is dispatched by.
pub const DISPLAY: &str = "whichever::foreign::Display";

/// The trait `whichever::foreign::Display` implements, as the hand names
/// it.
pub const CORE_DISPLAY: &str = "core::fmt::Display";

/// The declaration's description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclarationDescriptor {
    /// Whether the trait is declared without the attribute and restated
    /// beside the subject with `#[whichever::dispatchable(remote = ..)]`.
    pub restated: bool,
    /// Whether the restatement lists the method under test; one that does
    /// not leaves it to the real trait's default.
    pub lists_method: bool,
    /// Whether the trait is `whichever::foreign::Display`.
    pub foreign: bool,
}

impl Declaration {
    /// Returns the declaration's description.
    pub fn descriptor(self) -> DeclarationDescriptor {
        let dispatchable = DeclarationDescriptor {
            restated: false,
            lists_method: true,
            foreign: false,
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
                foreign: true,
                ..dispatchable
            },
        }
    }
}
