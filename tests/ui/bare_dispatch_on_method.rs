//! A method's `#[dispatch]` names the variants to forward to.
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
    #[dispatch]
    fn radius(&self) -> u32 {
        0
    }
}

fn main() {}
