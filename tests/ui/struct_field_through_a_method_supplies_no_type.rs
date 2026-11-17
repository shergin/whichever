//! A struct's field reached through a method has no type to name, so it
//! cannot supply an associated type; the sentence speaks of the struct and
//! its field, and the way is the override on the struct.
#[whichever::dispatchable]
pub trait Shape {
    type Unit;
    fn area(&self) -> f64;
}

pub struct Square(pub f64);

impl Shape for Square {
    type Unit = u8;
    fn area(&self) -> f64 {
        self.0 * self.0
    }
}

pub struct Holder {
    pub square: Square,
}

impl Holder {
    pub fn square(&self) -> &Square {
        &self.square
    }
}

#[whichever::dispatch(Shape)]
pub struct Wrapped {
    #[dispatch(via = .square())]
    pub holder: Holder,
}

fn main() {}
