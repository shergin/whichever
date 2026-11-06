//! `into` converts every variant's payload, so a variant with other fields
//! is reported.
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

struct Printed(u32);

impl From<Circle> for Printed {
    fn from(circle: Circle) -> Self {
        Printed(circle.0)
    }
}

#[whichever::dispatch(Shape, into = Printed)]
enum Kind {
    Circle(Circle),
    Tagged(#[dispatch] Circle, u32),
}

fn main() {}
