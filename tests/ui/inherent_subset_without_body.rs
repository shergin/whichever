#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

struct Circle;
struct Square;

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

impl Shape for Square {
    fn area(&self) -> u32 {
        1
    }
}

#[whichever::dispatch(Shape)]
enum AnyShape {
    Circle(Circle),
    Square(Square),
}

#[whichever::dispatch]
impl AnyShape {
    #[dispatch(Circle)]
    fn radius(&self) -> u32;
}

fn main() {}
