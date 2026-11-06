//! A `Box<Self>` receiver on a variant holding a `Box` lacks only `deref`.
#[whichever::dispatchable]
trait Consume {
    fn consume(self: Box<Self>) -> u32;
}

struct Circle(u32);

impl Consume for Circle {
    fn consume(self: Box<Self>) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Consume)]
enum Kind {
    Boxed(Box<Circle>),
}

fn main() {}
