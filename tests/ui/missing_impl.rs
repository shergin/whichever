#[whichever::dispatchable]
trait Value {
    fn value(&self) -> u32;
}

struct A;
struct B;

impl Value for A {
    fn value(&self) -> u32 {
        1
    }
}

#[whichever::dispatch(Value)]
enum Kind {
    A(A),
    B(B),
}

fn main() {}
