//! A trait object has no function to call without a value, so a required
//! method without a receiver is reported on a struct holding one.
#[whichever::dispatchable]
trait Shape {
    fn kind() -> &'static str
    where
        Self: Sized;
    fn area(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn kind() -> &'static str {
        "square"
    }
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape)]
struct Wrapper(#[dispatch(deref)] Box<dyn Shape>);

fn main() {}
