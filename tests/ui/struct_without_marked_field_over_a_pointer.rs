//! A struct that marks no field, with a field spelled as a pointer, is
//! told of `#[dispatch(deref)]`, the mark that forwards to what the
//! pointer points at. An unnamed field is named by its index.
use std::rc::Rc;

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
}

pub struct Circle(u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Shape)]
pub struct Shared(u8, Rc<Circle>);

pub fn area(shape: &Shared) -> u32 {
    shape.area()
}

fn main() {}
