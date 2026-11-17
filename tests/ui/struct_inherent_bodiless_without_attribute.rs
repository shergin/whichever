//! A bodiless method of a struct's inherent block without a mark says how
//! to forward it to the struct's field.
pub struct Square(pub u32);

impl Square {
    pub fn side(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
pub struct Wrapper(Square);

#[whichever::dispatch]
impl Wrapper {
    pub fn side(&self) -> u32;
}

fn main() {}
