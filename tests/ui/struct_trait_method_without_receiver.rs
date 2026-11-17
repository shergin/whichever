//! A trait method without a receiver has no field to forward to on a
//! struct.
#[whichever::dispatchable]
trait Shape {
    fn kind() -> &'static str;
    fn area(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn kind() -> &'static str {
        "square"
    }
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape)]
struct Wrapper(Square);

fn main() {}
