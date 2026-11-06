//! A route is made of `*`, `.name`, `.0` and `.name()`.
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
enum Kind {
    #[dispatch(via = base)]
    Circle(Circle),
}

fn main() {}
