//! A variant is the target of every trait its enum dispatches, so its field
//! names none; a trait takes its own route through the variant on the enum.
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

#[whichever::dispatch(Shape)]
enum Any {
    Tagged {
        #[dispatch(Shape)]
        inner: Circle,
        n: u32,
    },
    Plain(Circle),
}

fn main() {}
