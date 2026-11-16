//! An arm of a method under `#[dispatch(*)]` names the variants it answers,
//! and every other variant is forwarded: a wildcard names none, and would
//! leave nothing to forward.
pub struct IntBounds(pub u32);

impl IntBounds {
    pub fn len(&self) -> usize {
        1
    }
}

#[whichever::dispatch]
pub enum Bounds {
    Int(IntBounds),
    NoOrder,
}

#[whichever::dispatch]
impl Bounds {
    #[dispatch(*)]
    pub fn len(&self) -> usize {
        match self {
            Self::NoOrder => 0,
            _ => 1,
        }
    }
}

fn main() {}
