//! Another crate's enum gets no inherent impl here.
pub mod other {
    pub enum Pair<L, R> {
        Left(L),
        Right(R),
    }
}

use other::Pair;

#[whichever::dispatch(remote = other::Pair)]
pub enum Pair<L, R> {
    Left(L),
    Right(R),
}

#[whichever::dispatch]
impl<L, R> Pair<L, R> {
    #[dispatch(*)]
    pub fn describe(&self) -> String;
}

fn main() {}
