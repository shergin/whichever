//! A variant and its field give one route between them.
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
    #[dispatch(deref)]
    Boxed(#[dispatch(via = *)] Box<Circle>),
}

fn main() {}
