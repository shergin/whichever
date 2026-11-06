//! `#[dispatch(default)]` leaves the method to the trait's default, so it
//! takes no body.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
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
impl Shape for Kind {
    #[dispatch(default)]
    fn name(&self) -> &'static str {
        "kind"
    }
}

fn main() {}
