//! A partial block's method on a struct that names a field other than the
//! one its trait is forwarded to is told where the trait goes, and how to
//! send it to the field it names.
#[whichever::dispatchable]
trait Catalog {
    fn size(&self) -> usize;
}

struct Books;

impl Catalog for Books {
    fn size(&self) -> usize {
        1
    }
}

#[whichever::dispatch]
struct Env<C, N> {
    #[dispatch(Catalog)]
    catalog: C,
    #[dispatch]
    named: N,
}

#[whichever::dispatch]
impl<C: Catalog, N> Catalog for Env<C, N> {
    #[dispatch(named)]
    fn size(&self) -> usize;
}

fn main() {}
