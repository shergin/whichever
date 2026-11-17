//! An unknown option on a partial impl block's attribute lists the trait's
//! options, the ones a block takes.
#[whichever::dispatchable]
trait Value {
    type Unit;
    fn value(&self) -> u32;
}

struct A;

impl Value for A {
    type Unit = u8;
    fn value(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
enum Kind {
    A(A),
}

#[whichever::dispatch(Unit = u8, inline)]
impl Value for Kind {}

fn main() {}
