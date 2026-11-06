//! `deref` is the route `*`.
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
    #[dispatch(deref, via = .0)]
    Boxed(Box<Circle>),
}

fn main() {}
