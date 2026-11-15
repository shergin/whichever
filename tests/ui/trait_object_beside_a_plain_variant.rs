//! Each variant a method cannot reach is named in a sentence of its own:
//! under `self: Box<Self>`, `Plain` holds a plain `Square`, and `Object`
//! holds a trait object under a method that requires `Self: Sized`. The
//! trait object's sentence is its variant's arm, as the other is.

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn take(self: Box<Self>) -> u32
    where
        Self: Sized;
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

#[whichever::dispatch(Shape)]
pub enum Both {
    Plain(Square),
    #[dispatch(deref)]
    Object(Box<dyn Shape>),
}

fn main() {}
