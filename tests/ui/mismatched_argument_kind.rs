#[whichever::dispatchable]
trait Convert<X> {
    fn convert(&self, x: X) -> X;
}

struct A;

impl Convert<u32> for A {
    fn convert(&self, x: u32) -> u32 {
        x
    }
}

#[whichever::dispatch(Convert<'static>)]
enum Kind {
    A(A),
}

fn main() {}
