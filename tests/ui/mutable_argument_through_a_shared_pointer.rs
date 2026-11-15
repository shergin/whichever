//! A `&mut Self` argument on a `deref` variant over an `Rc` would borrow
//! mutably what the pointer lends shared only; the sentence names the
//! argument.
use std::rc::Rc;

#[whichever::dispatchable]
pub trait Absorb {
    #[dispatch(mismatch = ())]
    fn absorb(&self, other: &mut Self);
}

pub struct Square(pub u64);

impl Absorb for Square {
    fn absorb(&self, other: &mut Self) {
        other.0 += self.0;
    }
}

#[whichever::dispatch(Absorb)]
pub enum Shape {
    #[dispatch(deref)]
    Shared(Rc<Square>),
}

fn main() {}
