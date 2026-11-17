//! A generic associated type is declared by the struct in a partial impl
//! block; the sentence names the struct.
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

#[whichever::dispatch(Lend)]
struct Wrapper(A);

fn main() {}
