//! A marked unnamed field that is the last of its variant or struct is
//! matched without a rest pattern, which would have nothing to skip; a
//! marked field with others after it keeps its `..`.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u64;
}

pub struct Square(pub u64);

impl Shape for Square {
    fn area(&self) -> u64 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape)]
pub enum Tagged {
    Last(u64, #[dispatch] Square),
    Middle(u8, #[dispatch] Square, u64),
}

#[whichever::dispatch(Shape)]
pub struct Labelled(u64, #[dispatch] Square);

fn main() {}
