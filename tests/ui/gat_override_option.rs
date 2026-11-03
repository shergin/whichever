//! A generic associated type cannot be fixed with an option; the partial
//! impl block declares it.
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
    fn lend(&self) -> &A {
        self
    }
}

#[whichever::dispatch(Lend, Item<'a> = &'a A)]
enum Kind {
    A(A),
}

fn main() {}
