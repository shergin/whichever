//! A partial block of a trait the enum's own attribute dispatches would be
//! a second impl; it is reported on the block, and the attribute's impl
//! stands, so the calls resolve.
#[whichever::dispatchable]
trait Value {
    fn value(&self) -> u32;
    fn doubled(&self) -> u32 {
        self.value() * 2
    }
}

struct A;

impl Value for A {
    fn value(&self) -> u32 {
        1
    }
}

#[whichever::dispatch(Value)]
enum Kind {
    A(A),
}

#[whichever::dispatch]
impl Value for Kind {
    fn doubled(&self) -> u32 {
        0
    }
}

fn main() {
    let _ = Kind::A(A).doubled();
}
