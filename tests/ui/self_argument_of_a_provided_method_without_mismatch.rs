//! A provided method with a `Self` argument has a third answer beside the
//! two `mismatch` forms, which looks at both values: the trait's default.
//! The sentence names it in a partial block for the enum's attribute, and
//! in this block for a block's method.
#[whichever::dispatchable]
pub trait Same {
    fn key(&self) -> u8;
    fn same(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

pub struct A(pub u8);
pub struct B(pub u8);

impl Same for A {
    fn key(&self) -> u8 {
        self.0
    }
}

impl Same for B {
    fn key(&self) -> u8 {
        self.0
    }
}

#[whichever::dispatch(Same)]
pub enum Kind {
    A(A),
    B(B),
}

#[whichever::dispatch]
pub enum Other {
    A(A),
    B(B),
}

#[whichever::dispatch]
impl Same for Other {}

fn main() {}
