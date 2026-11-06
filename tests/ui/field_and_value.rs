//! `field` reads a field and `value` is the target itself.
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
    #[dispatch(*, field = 0, value)]
    fn radius(&self) -> u32;
}

fn main() {}
