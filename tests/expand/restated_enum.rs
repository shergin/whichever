//! An enum another crate defines is restated with `remote`, and the impl
//! is written for the real path.
pub mod other {
    pub enum Pair<L, R> {
        Left(L),
        Right(R),
    }
}

#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u32;
}

#[whichever::dispatch(remote = other::Pair)]
#[whichever::dispatch(Area)]
pub enum Pair<L, R> {
    Left(L),
    Right(R),
}

fn main() {}
