//! An associated type is fixed with a type.
#[whichever::dispatchable]
trait Store {
    type Item;
    fn first(&self) -> Self::Item;
}

struct Narrow;

impl Store for Narrow {
    type Item = u32;
    fn first(&self) -> u32 {
        1
    }
}

#[whichever::dispatch(Store, Item = 1 + 2)]
enum Kind {
    Narrow(Narrow),
}

fn main() {}
