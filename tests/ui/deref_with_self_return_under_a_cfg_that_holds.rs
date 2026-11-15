//! A variant the method cannot be forwarded to is reported where its `cfg`
//! holds; `#[cfg(all())]` stands for a test-only variant in a test build.
#[whichever::dispatchable]
pub trait Grow {
    fn grown(&self) -> Self;
}

pub struct Square(pub u64);

impl Grow for Square {
    fn grown(&self) -> Self {
        Square(self.0 + 1)
    }
}

#[whichever::dispatch(Grow)]
pub enum Shape {
    Plain(Square),
    #[cfg(all())]
    #[dispatch(deref)]
    Boxed(Box<Square>),
}

fn main() {}
