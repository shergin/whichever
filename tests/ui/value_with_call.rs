//! `value` calls no method.
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
    #[dispatch(*, value, call = area)]
    fn circle(&self) -> &Circle;
}

fn main() {}
