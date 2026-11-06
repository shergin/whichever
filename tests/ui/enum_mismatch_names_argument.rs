//! A `mismatch` written on the enum cannot see the trait method's
//! parameters: the trait's tokens reach the enum through a `macro_rules!`
//! body and carry its hygiene. One written on the trait method can.
#[whichever::dispatchable]
trait Same {
    fn same(&self, other: &Self) -> bool;
}

struct A;
struct B;

impl Same for A {
    fn same(&self, _: &Self) -> bool {
        true
    }
}

impl Same for B {
    fn same(&self, _: &Self) -> bool {
        true
    }
}

#[whichever::dispatch(Same, mismatch(same = other.is_a()))]
enum Kind {
    A(A),
    B(B),
}

impl Kind {
    fn is_a(&self) -> bool {
        true
    }
}

fn main() {}
