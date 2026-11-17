//! A struct another crate defines is restated with `remote`, and the impls
//! are written for the real path; the patterns end in `..`, since the real
//! struct may have fields the restatement leaves out.
pub mod other {
    pub struct Square(pub u32);
    pub struct Solo(pub Square);
    pub struct Pair {
        pub left: Square,
        pub right: u8,
        pub note: &'static str,
    }
}

use other::Square;

#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u32;
    fn grown(&self) -> Self;
}

#[whichever::dispatchable]
pub trait Tag {
    fn tag(&self) -> u8;
}

#[whichever::dispatch(Area, remote = other::Solo)]
pub struct Solo(Square);

#[whichever::dispatch(remote = other::Pair)]
#[whichever::dispatch(Tag)]
pub struct Pair {
    #[dispatch]
    left: Square,
    #[dispatch(Tag)]
    right: u8,
}

fn main() {}
