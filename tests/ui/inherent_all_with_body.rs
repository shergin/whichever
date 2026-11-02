#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

struct Circle;

impl Circle {
    fn radius(&self) -> u32 {
        1
    }
}

impl Shape for Circle {
    fn area(&self) -> u32 {
        3
    }
}

#[whichever::dispatch(Shape)]
enum AnyShape {
    Circle(Circle),
}

#[whichever::dispatch]
impl AnyShape {
    #[dispatch(*)]
    fn radius(&self) -> u32 {
        0
    }
}

fn main() {}
