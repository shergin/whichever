//! An override naming nothing in a trait without associated items, on a
//! struct: no conversion is offered, and the struct's other option is a
//! route.
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

#[whichever::dispatch(Value, form = false)]
struct Wrapper(A);

fn main() {}
