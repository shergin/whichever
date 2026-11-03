//! Every method that cannot be forwarded is reported, not only the first.
use std::rc::Rc;

#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
    fn shared(self: Rc<Self>) -> u32;
    fn points(&self) -> impl Iterator<Item = u32>;
}

struct Square;

impl Shape for Square {
    fn area(&self) -> u32 {
        1
    }
    fn shared(self: Rc<Self>) -> u32 {
        1
    }
    fn points(&self) -> impl Iterator<Item = u32> {
        std::iter::empty()
    }
}

#[whichever::dispatch(Shape)]
enum Kind {
    Square(Square),
}

fn main() {}
