//! A `mismatch` on an enum's attribute that names no trait is reported, and
//! the sentence for the block's `Self` argument names the block's
//! attribute, where the option goes.
#[whichever::dispatchable]
pub trait Same {
    fn same(&self, other: &Self) -> bool;
}

pub struct A(pub u8);
pub struct B(pub u8);

impl Same for A {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Same for B {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[whichever::dispatch(mismatch(same = false))]
pub enum Any {
    A(A),
    B(B),
}

#[whichever::dispatch]
impl Same for Any {}

fn main() {}
