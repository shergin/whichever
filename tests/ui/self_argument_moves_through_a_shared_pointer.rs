//! A `Self` argument by value through a `deref` variant over an `Rc` is
//! what would move, and the sentence names it, not the `&self` receiver.
use std::rc::Rc;

#[whichever::dispatchable]
pub trait Merge {
    #[dispatch(mismatch = 0)]
    fn merge(&self, other: Self) -> u64;
}

pub struct Square(pub u64);

impl Merge for Square {
    fn merge(&self, other: Self) -> u64 {
        self.0 + other.0
    }
}

#[whichever::dispatch(Merge)]
pub enum Shape {
    #[dispatch(deref)]
    Shared(Rc<Square>),
}

fn main() {}
