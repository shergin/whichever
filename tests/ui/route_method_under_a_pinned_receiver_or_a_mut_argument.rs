//! A route ending in a method call yields a value, which neither
//! `self: Pin<&mut Self>` nor a `&mut Self` argument can borrow through;
//! each sentence names what its method takes.
use std::pin::Pin;

#[whichever::dispatchable]
pub trait Grow {
    fn grow(self: Pin<&mut Self>);
    #[dispatch(mismatch = ())]
    fn absorb(&self, other: &mut Self);
}

pub struct Square(pub u64);

impl Grow for Square {
    fn grow(mut self: Pin<&mut Self>) {
        self.0 += 1;
    }
    fn absorb(&self, other: &mut Self) {
        other.0 += self.0;
    }
}

pub struct Account {
    base: Square,
}

impl Account {
    pub fn base(&self) -> &Square {
        &self.base
    }
}

#[whichever::dispatch(Grow)]
pub enum Any {
    #[dispatch(via = .base())]
    Module(Account),
}

fn main() {}
