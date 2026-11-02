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

#[whichever::dispatch(Same)]
enum Kind {
    A(A),
    B(B),
}

fn main() {}
