//! The generated code must name only `core` items, and the hidden items
//! must stay out of the public documentation.
#![no_std]
#![deny(missing_docs)]

/// Something with a size.
#[whichever::dispatchable]
pub trait Sized2 {
    /// Returns the size.
    fn size(&self) -> usize;
}

/// A fixed size.
pub struct Fixed(pub usize);

impl Sized2 for Fixed {
    fn size(&self) -> usize {
        self.0
    }
}

/// Twice a fixed size.
pub struct Double(pub usize);

impl Sized2 for Double {
    fn size(&self) -> usize {
        self.0 * 2
    }
}

/// Either kind of size.
#[whichever::dispatch(Sized2)]
pub enum Size {
    /// Fixed.
    Fixed(Fixed),
    /// Doubled.
    Double(Double),
}
