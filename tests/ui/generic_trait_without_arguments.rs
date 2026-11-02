#[whichever::dispatchable]
trait Convert<X> {
    fn convert(&self, x: X) -> X;
}

struct A;
struct B;

impl Convert<u32> for A {
    fn convert(&self, x: u32) -> u32 {
        x
    }
}

impl Convert<u32> for B {
    fn convert(&self, x: u32) -> u32 {
        x
    }
}

#[whichever::dispatch(Convert)]
enum Kind {
    A(A),
    B(B),
}

fn main() {}
