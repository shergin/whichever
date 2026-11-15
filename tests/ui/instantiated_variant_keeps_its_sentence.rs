//! A partial block implementing a trait for one instantiation of a generic
//! enum, whose method cannot be forwarded to the variant holding the
//! parameter: the sentence is on that variant's payload, where the
//! parameter is, and not on the header's argument that instantiates it.

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn take(self: Box<Self>) -> u32;
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn take(self: Box<Self>) -> u32 {
        self.0
    }
}

#[whichever::dispatch(from = false)]
pub enum Any<T> {
    Plain(T),
    #[dispatch(deref)]
    Boxed(Box<Square>),
}

#[whichever::dispatch]
impl Shape for Any<Square> {
    #[dispatch(Plain)]
    fn take(self: Box<Self>) -> u32 {
        0
    }
}

fn main() {}
