//! A return-type limit on a struct's method that has a body offers that
//! body as the way out, which answers by itself once the mark is gone.
pub struct Circle(pub u32);

impl Circle {
    pub fn grown(&self) -> Option<Circle> {
        Some(Circle(self.0 + 1))
    }
}

#[whichever::dispatch]
pub struct Labelled {
    #[dispatch]
    circle: Circle,
    pub label: &'static str,
}

#[whichever::dispatch]
impl Labelled {
    #[dispatch(circle, with = Some)]
    pub fn grown(&self) -> Option<Option<Self>> {
        None
    }
}

fn main() {}
