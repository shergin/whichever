//! A `&mut self` method on a `deref` variant over an `Rc` is a forwarding no
//! hand can write either, and rustc's error stands on the enum's name, where
//! the arm is, not on the trait's attribute.
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
