#[whichever::dispatchable]
trait Value {
    fn value(&self) -> u32;
}

struct A;

impl Value for A {
    fn value(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
enum Kind {
    A(A),
}

#[whichever::dispatch(from = false)]
impl Value for Kind {}

fn main() {}
