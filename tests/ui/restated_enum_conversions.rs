//! A restated enum gets no conversions.
pub mod other {
    pub enum Pair<L, R> {
        Left(L),
        Right(R),
    }
}

#[whichever::dispatch(remote = other::Pair, from = false)]
pub enum Pair<L, R> {
    Left(L),
    Right(R),
}

fn main() {}
