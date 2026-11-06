//! One `remote`.
mod other {
    pub enum Pair<L, R> {
        Left(L),
        Right(R),
    }
}

struct A;
struct B;

#[whichever::dispatch(remote = other::Pair, remote = other::Pair)]
enum Pair {
    Left(A),
    Right(B),
}

fn main() {}
