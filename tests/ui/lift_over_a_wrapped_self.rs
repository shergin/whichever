//! Under `with = Some` or `with = Ok`, the payload returns the plain value
//! and the lift wraps the rebuilt variant once: a declared return that
//! wraps `Self` twice is reported. The method names its variants and has a
//! body for the rest, so the way out is that body for every variant.
pub struct Circle(pub u32);

impl Circle {
    pub fn grown(&self) -> Option<Circle> {
        Some(Circle(self.0 + 1))
    }
}

#[whichever::dispatch]
pub enum Shape {
    Circle(Circle),
    Nothing,
}

#[whichever::dispatch]
impl Shape {
    #[dispatch(Circle, with = Some)]
    pub fn grown(&self) -> Option<Option<Self>> {
        None
    }
}

fn main() {}
