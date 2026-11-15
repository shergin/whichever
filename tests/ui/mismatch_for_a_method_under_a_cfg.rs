//! A `mismatch` on the enum names an item the trait has in every build: a
//! method under a `#[cfg]` that does not hold here is not in this build's
//! trait, and the sentence says where such a method takes its mismatch.
//! `#[cfg(any())]` stands for a feature left off. The second enum's trait
//! has another method taking a `Self` argument, which the sentence lists.
#[whichever::dispatchable]
pub trait Same {
    #[cfg(any())]
    fn same(&self, other: &Self) -> bool;
}

#[whichever::dispatchable]
pub trait Compare {
    #[dispatch(mismatch = false)]
    fn equal(&self, other: &Self) -> bool;
    #[cfg(any())]
    fn ordered_before(&self, other: &Self) -> bool;
}

pub struct Square(pub u64);

impl Same for Square {}

impl Compare for Square {
    fn equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[whichever::dispatch(Same, mismatch(same = false))]
pub enum Shape {
    Square(Square),
}

#[whichever::dispatch(Compare, mismatch(ordered_before = false))]
pub enum Figure {
    Square(Square),
}

fn main() {}
