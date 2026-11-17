//! A generic associated type cannot be fixed with an option on a struct's
//! attribute either; the sentence names the struct.
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
struct Wrapper(A);

fn main() {}
