//! A field names a trait as a partial block's header spells it; one name
//! spelled with another path is told on the block, and the impl is written
//! for the field that was meant, so the sentence stands alone: no error on
//! the bare field, which does not implement the trait, and none at the
//! call.
mod traits {
    #[whichever::dispatchable]
    pub trait Catalog {
        fn size(&self) -> usize;
    }
}

use traits::Catalog;

struct Books;

impl Catalog for Books {
    fn size(&self) -> usize {
        1
    }
}

struct Label;

#[whichever::dispatch]
struct Env<C, N> {
    #[dispatch(Catalog)]
    catalog: C,
    #[dispatch]
    named: N,
}

#[whichever::dispatch]
impl<C: Catalog, N> traits::Catalog for Env<C, N> {}

fn size(env: &Env<Books, Label>) -> usize {
    env.size()
}

fn main() {}
