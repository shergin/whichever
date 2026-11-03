//! A payload whose generic associated type does not convert into the one
//! the block declares fails on rustc's `Into` bound, like a payload whose
//! plain associated type does not convert into an override.
#[whichever::dispatchable]
trait Lend {
    type Item<'a>
    where
        Self: 'a;
    fn lend(&self) -> Self::Item<'_>;
}

struct A;
struct B;

impl Lend for A {
    type Item<'a> = &'a A;
    fn lend(&self) -> &A {
        self
    }
}

impl Lend for B {
    type Item<'a> = u32;
    fn lend(&self) -> u32 {
        1
    }
}

enum AnyItem<'a> {
    A(&'a A),
}

impl<'a> From<&'a A> for AnyItem<'a> {
    fn from(a: &'a A) -> Self {
        AnyItem::A(a)
    }
}

#[whichever::dispatch]
enum Kind {
    A(A),
    B(B),
}

#[whichever::dispatch]
impl Lend for Kind {
    type Item<'a> = AnyItem<'a> where Self: 'a;
}

fn main() {}
