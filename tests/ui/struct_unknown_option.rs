//! An unknown option on a struct's attribute lists a struct's options,
//! without the conversions and the mismatch, which are an enum's.
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

#[whichever::dispatch(Value, inline)]
struct Wrapper(A);

fn main() {}
