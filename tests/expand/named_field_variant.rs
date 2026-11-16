//! A variant whose one field is named and marked, beside a tuple variant:
//! a method returning `Self` rebuilds it by its braces, `Any::Named { inner:
//! .. }`, where the tuple variant is rebuilt by its name, and only the tuple
//! variant converts from its payload.
#[whichever::dispatchable]
pub trait Shape {
    fn radius(&self) -> u32;
    fn doubled(self) -> Self;
}

pub struct Circle(u32);

impl Shape for Circle {
    fn radius(&self) -> u32 {
        self.0
    }
    fn doubled(self) -> Self {
        Circle(self.0 * 2)
    }
}

#[whichever::dispatch(Shape)]
pub enum Any {
    Named {
        #[dispatch]
        inner: Circle,
    },
    Plain(Circle),
}

fn main() {}
