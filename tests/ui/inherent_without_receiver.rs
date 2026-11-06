//! An inherent method without a receiver has no variant to forward to.
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
    #[dispatch(*)]
    fn unit() -> u32;
}

fn main() {}
