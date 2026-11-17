//! An option of a trait's impl on an enum's attribute that names no trait
//! is reported on the option, naming the two places it goes. Beside the one
//! trait the enum dispatches it is read as meant, so the sentence stands
//! alone.
#[whichever::dispatchable]
pub trait Shape {
    const SIDES: u8;
    fn area(&self) -> u32;
    #[dispatch(mismatch = false)]
    fn same(&self, other: &Self) -> bool;
}

pub struct A(pub u32);
pub struct B(pub u32);

impl Shape for A {
    const SIDES: u8 = 4;
    fn area(&self) -> u32 {
        self.0
    }
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Shape for B {
    const SIDES: u8 = 4;
    fn area(&self) -> u32 {
        self.0
    }
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

pub struct Holder {
    pub shape: A,
}

#[whichever::dispatch(Shape)]
#[whichever::dispatch(SIDES = 7)]
pub enum Fixed {
    A(A),
    B(B),
}

#[whichever::dispatch(Shape)]
#[whichever::dispatch(mismatch(same = true))]
pub enum Mismatched {
    A(A),
    B(B),
}

#[whichever::dispatch(Shape)]
#[whichever::dispatch(via(Held = .shape))]
pub enum Routed {
    Held(Holder),
    B(B),
}

fn main() {}
