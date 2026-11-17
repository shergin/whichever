//! A home that spells its trait from `self`, beside a field naming it
//! without: the sentence stands alone, since the impl is written for the
//! field that was meant with the home's spelling, and the call resolves.
#[whichever::dispatchable]
pub trait Catalog {
    fn size(&self) -> usize;
}

struct Books;

impl Catalog for Books {
    fn size(&self) -> usize {
        1
    }
}

struct Label;

#[whichever::dispatch(self::Catalog)]
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
