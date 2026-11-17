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

/// What needs no value is the field's type's: a function, `-> Self` built
/// around it, a provided function, and a generic associated type, whose
/// own `T` is renamed beside the struct's.
#[whichever::dispatchable]
pub trait Kind {
    type Paired<T>;
    fn kind() -> &'static str;
    fn unit() -> Self;
    fn label() -> &'static str {
        "kind"
    }
    fn paired<T>(&self, with: T) -> Self::Paired<T>;
}

#[whichever::dispatch(Kind)]
pub struct Generic<T>(T);

#[whichever::dispatch]
pub struct Number(u128);

#[whichever::dispatch]
impl Number {
    #[dispatch(*, call = from)]
    pub fn from_u64(v: u64) -> Self;
}

fn main() {}
