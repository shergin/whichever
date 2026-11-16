//! A trait method with a `Self` argument is matched beside `self`, so an
//! arm written for `match self` alone is not one of its arms.
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
}

#[whichever::dispatch(mismatch(same = false))]
impl Same for Shape {
    #[dispatch(*)]
    fn same(&self, other: &Self) -> bool {
        match self {
            Self::Nothing => matches!(other, Self::Nothing),
        }
    }
}

fn main() {}
