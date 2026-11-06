//! One field takes every trait not named elsewhere.
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
struct Pair {
    #[dispatch]
    left: Circle,
    #[dispatch]
    right: Circle,
}

fn main() {}
