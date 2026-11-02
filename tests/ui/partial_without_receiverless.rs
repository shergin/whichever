#[whichever::dispatchable]
trait Make {
    fn make(seed: u32) -> Self
    where
        Self: Sized;
    fn value(&self) -> u32;
}

struct A;

impl Make for A {
    fn make(_seed: u32) -> Self {
        A
    }
    fn value(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
enum Kind {
    A(A),
}

#[whichever::dispatch]
impl Make for Kind {}

fn main() {}
