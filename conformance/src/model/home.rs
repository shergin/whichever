//! What a home is: where the trait and the payloads are declared, and how
//! the trait's signature names a type of its own crate (limits.md: the
//! types in the trait's signatures are written into the enum's module, and
//! `crate::` paths are rewritten to the trait's crate).

use super::axes::Home;

/// A home's description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomeDescriptor {
    /// The module of the cell the trait is declared in, beside the
    /// subject; `None` for the cell's own module.
    pub module: Option<&'static str>,
    /// Whether the trait and the payloads are in a crate of their own,
    /// which the subject's crate depends on.
    pub foreign: bool,
    /// The type of the method's `u64` argument as the trait spells it.
    pub argument: &'static str,
}

impl Home {
    /// Returns the home's description.
    pub fn descriptor(self) -> HomeDescriptor {
        let crate_path = "crate::support::Count";
        // A type only the trait's crate declares, at its root: left as
        // `crate::Tally` in the subject's crate it names nothing there, so
        // only the rewrite to the trait's crate makes it resolve (limits.md,
        // the types in the trait's signatures).
        let own_path = "crate::Tally";
        match self {
            Home::Local => HomeDescriptor {
                module: None,
                foreign: false,
                argument: "u64",
            },
            Home::Sibling => HomeDescriptor {
                module: Some("home"),
                foreign: false,
                argument: crate_path,
            },
            Home::Crate => HomeDescriptor {
                module: None,
                foreign: true,
                argument: own_path,
            },
        }
    }
}
