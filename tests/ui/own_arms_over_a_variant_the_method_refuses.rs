//! Under `#[dispatch(*)]` with arms of the method's own, a variant the
//! method cannot be forwarded to is answered by one more arm: the sentence
//! names that arm as the way around.
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

#[whichever::dispatch]
pub enum Shared {
    Owned(Square),
    #[dispatch(deref)]
    Counted(Rc<Square>),
    Empty,
}

#[whichever::dispatch]
impl Grow for Shared {
    #[dispatch(*)]
    fn grow(&mut self) -> u64 {
        match self {
            Self::Empty => 0,
        }
    }
}

fn main() {}
