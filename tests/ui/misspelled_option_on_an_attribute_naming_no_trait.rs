//! A name close to one of the enum's or the struct's own options, on an
//! attribute that names no trait, was probably that option.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
}

pub struct A(pub u32);

impl Shape for A {
    fn area(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Shape)]
#[whichever::dispatch(form = false)]
pub enum Misspelled {
    A(A),
}

#[whichever::dispatch(Shape)]
#[whichever::dispatch(remot = other::A)]
pub struct AlsoMisspelled {
    inner: A,
}

fn main() {}
