#[whichever::dispatchable]
trait Consume {
    fn consume(self: Box<Self>) -> u32;
}

struct A;

impl Consume for A {
    fn consume(self: Box<Self>) -> u32 {
        1
    }
}

#[whichever::dispatch(Consume)]
enum Kind {
    A(A),
}

fn main() {}
