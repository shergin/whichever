//! `via = route` after a trait is a route without the name it goes
//! through; the sentence says where a route goes.
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

struct Holder {
    square: Square,
}

#[whichever::dispatch(Shape, via = .square)]
struct Labelled {
    holder: Holder,
}

fn main() {}
