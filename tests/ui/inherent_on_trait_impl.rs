trait Shape {
    fn area(&self) -> u32;
}

struct Circle;

#[whichever::dispatch]
enum AnyShape {
    Circle(Circle),
}

#[whichever::dispatch]
impl Shape for AnyShape {
    fn area(&self) -> u32 {
        0
    }
}

fn main() {}
