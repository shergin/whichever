//! A field names a trait as the struct's attribute spells it; one name
//! spelled with another path is told, and the impl is written for the
//! field that was meant, so the sentence stands alone: no error on the bare
//! field, which does not implement the trait, and none at the call.
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

#[whichever::dispatch(traits::Catalog)]
struct Env {
    #[dispatch(Catalog)]
    catalog: Books,
    #[dispatch]
    named: Label,
}

fn size(env: &Env) -> usize {
    env.size()
}

fn main() {}
