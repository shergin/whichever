//! A variant named twice in a method's `#[dispatch(..)]` is reported.
struct Circle;

impl Circle {
    fn radius(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
enum Shape {
    Circle(Circle),
}

#[whichever::dispatch]
impl Shape {
    #[dispatch(Circle, Circle)]
    fn radius(&self) -> u32 {
        0
    }
}

fn main() {}
