//! A method without a body in an inherent block is the engine's to write.
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

#[whichever::dispatch]
impl Kind {
    fn area(&self) -> u32;
}

fn main() {}
