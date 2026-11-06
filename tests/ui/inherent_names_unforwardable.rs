//! An inherent method naming a variant without a payload is reported on
//! the name.
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
    Nothing,
}

#[whichever::dispatch]
impl Kind {
    #[dispatch(Nothing)]
    fn area(&self) -> u32 {
        0
    }
}

fn main() {}
