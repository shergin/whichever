//! A method on a route takes no arguments.
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

struct Holder(Vec<Circle>);

#[whichever::dispatch(Shape)]
enum Kind {
    #[dispatch(via = .0.get(1))]
    Held(Holder),
}

fn main() {}
