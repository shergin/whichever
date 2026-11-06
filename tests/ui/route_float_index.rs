//! A field index is a number.
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

#[whichever::dispatch(Shape)]
enum Kind {
    #[dispatch(via = .1e3)]
    Circle(Circle),
}

fn main() {}
