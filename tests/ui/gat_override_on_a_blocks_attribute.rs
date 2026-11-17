//! On a partial impl block's own attribute, a generic associated type is
//! the block's to declare; the sentence names neither an enum nor a
//! struct, since the block is read before either is known.
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

#[whichever::dispatch]
enum Kind {
    A(A),
}

#[whichever::dispatch(Item<'a> = &'a A)]
impl Lend for Kind {}

fn main() {}
