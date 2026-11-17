//! A trait's route on a struct keyed by a name the struct has no field
//! of, here the payload's type where the struct's field is meant.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape, via(Square = .square))]
struct Wrapped {
    square: Square,
}

fn main() {}
