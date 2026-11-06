//! A `.` on a route is followed by a field or a method.
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
    #[dispatch(via = ., from = false)]
    Circle(Circle),
}

fn main() {}
