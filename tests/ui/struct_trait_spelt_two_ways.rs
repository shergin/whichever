//! A field names a trait as the struct's attribute spells it; one name
//! spelled with another path is told, since the attribute would forward
//! to the bare field and the field's mark would be dead.
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

#[whichever::dispatch(traits::Catalog)]
struct Env {
    #[dispatch(Catalog)]
    catalog: Books,
    #[dispatch]
    named: Books,
}

fn main() {}
