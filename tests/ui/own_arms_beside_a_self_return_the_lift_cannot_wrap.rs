//! A method under `#[dispatch(*)]` whose body holds arms of its own, with a
//! return type the arms cannot wrap `Self` back into: the way out is every
//! variant's arm in the body, without the mark.
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
    #[dispatch(*, with = Some)]
    pub fn grown(&self) -> Option<Option<Self>> {
        match self {
            Self::Nothing => None,
        }
    }
}

fn main() {}
