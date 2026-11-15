//! `self: Box<Self>` on a `deref` variant over a pointer that is not a
//! `Box`: there is no box to hand over, and boxing the pointer would not
//! make one of what it points at.
use std::rc::Rc;

#[whichever::dispatchable]
pub trait Finish {
    fn finish(self: Box<Self>) -> u64;
}

pub struct Square(pub u64);

impl Finish for Square {
    fn finish(self: Box<Self>) -> u64 {
        self.0
    }
}

#[whichever::dispatch(Finish)]
pub enum Shape {
    #[dispatch(deref)]
    Boxed(Box<Square>),
    #[dispatch(deref)]
    Shared(Rc<Square>),
}

fn main() {}
