//! An associated type of one of the enum's type parameters may be any type,
//! so `TryFrom<Enum> for S::Out` would overlap the `TryFrom` that `core`
//! implements for every type; the message names the switch.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
}

pub trait Source {
    type Out: Shape;
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Shape, from = false, try_from = true)]
pub enum Plain<S: Source> {
    Out(S::Out),
    Square(Square),
}

fn main() {}
