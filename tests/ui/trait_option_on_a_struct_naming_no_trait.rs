//! An option of a trait's impl on a struct's attribute that names no trait
//! is reported on the option; beside the one trait the struct dispatches it
//! is read as meant, so the sentence stands alone.
#[whichever::dispatchable]
pub trait Shape {
    const SIDES: u8;
    fn area(&self) -> u32;
}

pub struct A(pub u32);

impl Shape for A {
    const SIDES: u8 = 4;
    fn area(&self) -> u32 {
        self.0
    }
}

pub struct Labelled {
    pub shape: A,
}

#[whichever::dispatch(Shape, SIDES = 4)]
#[whichever::dispatch(via(inner = .shape))]
pub struct Routed {
    inner: Labelled,
}

#[whichever::dispatch(Shape)]
#[whichever::dispatch(SIDES = 3)]
pub struct Fixed {
    inner: A,
}

fn main() {}
