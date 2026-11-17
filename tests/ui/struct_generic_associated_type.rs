//! A generic associated type is the struct's field's where the field's type
//! can be named; through a route it cannot, and the struct declares it in a
//! partial impl block, which the sentence names.
#[whichever::dispatchable]
trait Lend {
    type Item<'a>
    where
        Self: 'a;
    fn lend(&self) -> Self::Item<'_>;
}

struct A;

impl Lend for A {
    type Item<'a> = &'a A;
    fn lend(&self) -> Self::Item<'_> {
        self
    }
}

struct Holder {
    a: A,
}

#[whichever::dispatch(Lend)]
struct Wrapper {
    #[dispatch(via = .a)]
    holder: Holder,
}

fn main() {}
