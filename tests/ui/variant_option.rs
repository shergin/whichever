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

#[whichever::dispatch(Value)]
enum Kind {
    #[dispatch(boxed)]
    A(Box<A>),
}

fn main() {}
