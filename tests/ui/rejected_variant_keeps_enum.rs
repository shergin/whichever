//! The variant's shape is reported once, where a trait is dispatched, and
//! once more where `*` would forward to it; the derive, the `use` in another
//! module, the other inherent methods and the calls all resolve.
#[whichever::dispatchable]
trait Value {
    fn value(&self) -> u32;
}

#[derive(Debug)]
struct A;

impl Value for A {
    fn value(&self) -> u32 {
        1
    }
}

impl A {
    fn only_a(&self) -> u32 {
        1
    }
    fn bodyless(&self) -> u32 {
        1
    }
}

#[derive(Debug)]
#[whichever::dispatch(Value)]
enum Kind {
    A(A),
    B { inner: A },
}

#[whichever::dispatch]
impl Kind {
    #[dispatch(A)]
    fn only_a(&self) -> u32 {
        0
    }

    #[dispatch(*)]
    fn bodyless(&self) -> u32;

    pub fn plain(&self) -> u32 {
        2
    }
}

mod other {
    pub fn describe(kind: &super::Kind) -> String {
        format!("{kind:?}")
    }
}

fn main() {
    let kind = Kind::A(A);
    let _ = other::describe(&kind);
    let _ = kind.only_a() + kind.plain();
}
