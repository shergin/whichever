#[whichever::dispatchable]
trait Merge {
    fn merge(&self, other: Option<&Self>) -> u32;
}

struct A(u32);
struct B(u32);

impl Merge for A {
    fn merge(&self, other: Option<&Self>) -> u32 {
        self.0 + other.map_or(0, |other| other.0)
    }
}

impl Merge for B {
    fn merge(&self, other: Option<&Self>) -> u32 {
        self.0 + other.map_or(0, |other| other.0)
    }
}

#[whichever::dispatch(Merge)]
enum Kind {
    A(A),
    B(B),
}

fn main() {}
