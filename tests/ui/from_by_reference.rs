//! `from` is `true` or `false`.
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

#[whichever::dispatch(Shape, from = ref)]
enum Kind {
    Circle(Circle),
}

fn main() {}
