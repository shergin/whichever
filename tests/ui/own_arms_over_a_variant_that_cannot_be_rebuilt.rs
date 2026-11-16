//! A method returning `Self` under `#[dispatch(*)]` with arms of its own
//! cannot be forwarded to the marked field of a variant with several: the
//! way out is the arm of that variant in the body.
pub struct Square(pub u32);

impl Square {
    pub fn grown(&self) -> Square {
        Square(self.0 + 1)
    }
}

#[whichever::dispatch]
pub enum Shape {
    Tagged(#[dispatch] Square, u32),
    Plain(Square),
    Empty,
}

#[whichever::dispatch]
impl Shape {
    #[dispatch(*)]
    pub fn grown(&self) -> Self {
        match self {
            Self::Empty => Self::Empty,
        }
    }
}

fn main() {}
