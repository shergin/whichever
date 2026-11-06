//! An associated const is fixed with a value.
#[whichever::dispatchable]
trait Sized2 {
    const WIDTH: u32;
}

struct Circle;

impl Sized2 for Circle {
    const WIDTH: u32 = 1;
}

#[whichever::dispatch(Sized2, WIDTH = Vec<u8>)]
enum Kind {
    Circle(Circle),
}

fn main() {}
