//! A partial block over an enum whose only variant in the build holds no
//! payload: the variant's sentence stands alone, with no warning about the
//! method's arguments, which no arm is left to use.
#[whichever::dispatchable]
pub trait Grow {
    fn grow(&self, by: u64) -> u64;
}

pub struct Square(pub u64);

impl Grow for Square {
    fn grow(&self, by: u64) -> u64 {
        self.0 + by
    }
}

#[whichever::dispatch]
pub enum Shape {
    #[cfg(any())]
    Square(Square),
    Nothing,
}

#[whichever::dispatch]
impl Grow for Shape {}

fn main() {}
