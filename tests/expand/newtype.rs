#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn grown(&self) -> Self
    where
        Self: Sized;
}

pub struct Circle(pub u32);
pub struct Square(pub u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
    fn grown(&self) -> Self {
        Circle(self.0 + 1)
    }
}

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn grown(&self) -> Self {
        Square(self.0 + 1)
    }
}

#[whichever::dispatch(Shape)]
enum AnyShape {
    Circle(Circle),
    Square(Square),
}

/// The public wrapper over the private enum.
#[whichever::dispatch(Shape)]
pub struct Wrapped(AnyShape);

#[whichever::dispatch]
pub struct Labelled {
    #[dispatch]
    shape: AnyShape,
    label: &'static str,
}

#[whichever::dispatch]
impl Shape for Labelled {
    fn grown(&self) -> Self {
        Labelled {
            shape: self.shape.grown(),
            label: self.label,
        }
    }
}

fn main() {}
