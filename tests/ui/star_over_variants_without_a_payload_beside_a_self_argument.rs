//! `#[dispatch(*)]` over variants that hold no single payload, for a method
//! with a `Self` argument: each variant gets its sentence, and nothing else
//! is reported beside them.
#[whichever::dispatchable]
pub trait Same {
    fn same(&self, other: &Self) -> bool;
}

pub struct Square(pub u64);

impl Same for Square {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[whichever::dispatch]
pub enum Shape {
    Square(Square),
    Nothing,
    Pair(u64, u64),
}

#[whichever::dispatch(mismatch(same = false))]
impl Same for Shape {
    #[dispatch(*)]
    fn same(&self, other: &Self) -> bool;
}

fn main() {}
