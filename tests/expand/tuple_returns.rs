//! A tuple holding `Self` or an overridden associated type is taken apart,
//! each element converted, and built again.
#[whichever::dispatchable]
pub trait Split: Sized {
    type Rest;
    fn split(&self) -> (Self, u32);
    fn halves(&self) -> (Self::Rest, Option<Self>);
}

pub struct Even(pub u32);

impl Split for Even {
    type Rest = u8;
    fn split(&self) -> (Self, u32) {
        (Even(self.0 / 2), self.0 / 2)
    }
    fn halves(&self) -> (Self::Rest, Option<Self>) {
        (1, Some(Even(self.0)))
    }
}

#[whichever::dispatch(Split, Rest = u32)]
pub enum Number {
    Even(Even),
}

fn main() {}
