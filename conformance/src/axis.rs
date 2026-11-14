//! An axis: one feature of the model with the values it takes.
//!
//! An axis is a closed enum whose first value is the neutral one, the value
//! a cell holds when its id does not name the axis. Each value has a word,
//! the kebab-case spelling it has in ids. The [`axis!`](crate::axis!) macro
//! declares such an enum from `Variant = "word"` lines.

use core::fmt::Debug;

/// One feature of the model with the values it takes, the neutral first.
pub trait Axis: Copy + Eq + Debug + 'static {
    /// Every value, the neutral one first.
    const VALUES: &'static [Self];

    /// Returns the kebab-case word the value has in ids.
    fn word(self) -> &'static str;

    /// Returns the neutral value, the one a cell holds when its id does not
    /// name the axis.
    fn neutral() -> Self {
        Self::VALUES[0]
    }

    /// Returns the value's position in [`Axis::VALUES`].
    fn index(self) -> usize {
        Self::VALUES
            .iter()
            .position(|value| *value == self)
            .expect("every value of an axis is listed in its `VALUES`")
    }

    /// Returns the value whose word is `word`.
    fn from_word(word: &str) -> Option<Self> {
        Self::VALUES
            .iter()
            .copied()
            .find(|value| value.word() == word)
    }

    /// Returns the words of every value, the neutral one first.
    fn words() -> Vec<&'static str> {
        Self::VALUES.iter().map(|value| value.word()).collect()
    }
}

/// Declares an axis: an enum with one documented variant per value, the
/// neutral one first, and its [`Axis`] impl.
///
/// ```ignore
/// axis! {
///     /// The receiver of the method under test.
///     pub enum Receiver {
///         /// `&self`.
///         Shared = "ref",
///         /// `&mut self`.
///         Exclusive = "mut",
///     }
/// }
/// ```
#[macro_export]
macro_rules! axis {
    (
        $(#[$meta:meta])*
        $visibility:vis enum $name:ident {
            $(
                $(#[$value_meta:meta])*
                $value:ident = $word:literal,
            )+
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $visibility enum $name {
            $(
                $(#[$value_meta])*
                $value,
            )+
        }

        impl $crate::axis::Axis for $name {
            const VALUES: &'static [Self] = &[$($name::$value,)+];

            fn word(self) -> &'static str {
                match self {
                    $($name::$value => $word,)+
                }
            }
        }
    };
}
