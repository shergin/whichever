//! Another crate's struct gets no inherent impl here.
pub mod other {
    pub struct Square(pub u32);
    pub struct Solo(pub Square);
}

use other::Solo;

#[whichever::dispatch(remote = other::Solo)]
pub struct Solo(other::Square);

#[whichever::dispatch]
impl Solo {
    #[dispatch(*)]
    pub fn area(&self) -> u32;
}

fn main() {}
