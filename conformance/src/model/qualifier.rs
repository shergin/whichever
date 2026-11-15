//! What a qualifier is: how the method under test is declared.

use super::axes::Qualifier;

/// A type parameter of the method under test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generic {
    /// `fn m<X: Into<u64>>(.., x: X)`: the argument fixes `X`.
    Fixed,
    /// `fn m<W: Weight>(.., x: u64)`: no argument fixes `W`, so a caller
    /// spells it, `m::<W>(..)`.
    Turbofish,
}

/// A qualifier's description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualifierDescriptor {
    /// `async fn`.
    pub asyncness: bool,
    /// `unsafe fn`.
    pub unsafety: bool,
    /// The method's type parameter.
    pub generic: Option<Generic>,
    /// Whether the trait gives the method a default body, which `P0`
    /// overrides and `P1` does not.
    pub provided: bool,
}

impl Qualifier {
    /// Returns the qualifier's description.
    pub fn descriptor(self) -> QualifierDescriptor {
        let plain = QualifierDescriptor {
            asyncness: false,
            unsafety: false,
            generic: None,
            provided: false,
        };
        match self {
            Qualifier::Plain => plain,
            Qualifier::Async => QualifierDescriptor {
                asyncness: true,
                ..plain
            },
            Qualifier::Unsafe => QualifierDescriptor {
                unsafety: true,
                ..plain
            },
            Qualifier::Generic => QualifierDescriptor {
                generic: Some(Generic::Fixed),
                ..plain
            },
            Qualifier::Turbofish => QualifierDescriptor {
                generic: Some(Generic::Turbofish),
                ..plain
            },
            Qualifier::Provided => QualifierDescriptor {
                provided: true,
                ..plain
            },
        }
    }
}
