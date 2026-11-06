//! A partial block's attribute takes the trait's options only.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

struct Circle(u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
enum Kind {
    Circle(Circle),
}

#[whichever::dispatch(Shape)]
impl Shape for Kind {}

fn main() {}
