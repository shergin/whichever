//! A `&mut self` method on a `deref` variant over an `Rc` borrows mutably
//! what the pointer lends shared only, and is one sentence on the payload,
//! as a move out of the pointer is.
use std::rc::Rc;

#[whichever::dispatchable]
pub trait Grow {
    fn grow(&mut self) -> u64;
}

pub struct Square(pub u64);

impl Grow for Square {
    fn grow(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

#[whichever::dispatch(Grow)]
pub enum Shared {
    #[dispatch(deref)]
    Counted(Rc<Square>),
}

fn main() {}
