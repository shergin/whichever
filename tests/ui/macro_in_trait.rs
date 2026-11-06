//! A macro invocation inside the trait is not supported.
macro_rules! more {
    () => {
        fn more(&self) -> u32;
    };
}

#[whichever::dispatchable]
trait Shape {
    more!();
    fn area(&self) -> u32;
}

struct Circle(u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        self.0
    }
    fn more(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Shape)]
enum Kind {
    Circle(Circle),
}

fn main() {}
