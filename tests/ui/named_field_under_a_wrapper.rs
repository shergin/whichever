//! A dispatch struct whose one field is named cannot rebuild `Self` under
//! an `Option` or a `Result`: the value is mapped with a path, and no path
//! builds such a struct. `-> Self` beside them is rebuilt by the braces.
#[whichever::dispatchable]
pub trait Grow: Sized {
    fn grown(&self) -> Self;
    fn maybe_grown(&self) -> Option<Self>;
    fn tried_grown(&self) -> Result<Self, String>;
}

pub struct Square(pub u64);

impl Grow for Square {
    fn grown(&self) -> Self {
        Square(self.0 + 1)
    }
    fn maybe_grown(&self) -> Option<Self> {
        Some(self.grown())
    }
    fn tried_grown(&self) -> Result<Self, String> {
        Ok(self.grown())
    }
}

#[whichever::dispatch(Grow)]
pub struct Named {
    shape: Square,
}

fn main() {}
