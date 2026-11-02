#[whichever::dispatchable]
trait Factory {
    fn presets() -> Vec<u32>;
    fn id(&self) -> u32;
}

struct A;
struct B;

impl Factory for A {
    fn presets() -> Vec<u32> {
        vec![1]
    }
    fn id(&self) -> u32 {
        1
    }
}

impl Factory for B {
    fn presets() -> Vec<u32> {
        vec![2]
    }
    fn id(&self) -> u32 {
        2
    }
}

#[whichever::dispatch(Factory)]
enum Kind {
    A(A),
    B(B),
}

fn main() {}
