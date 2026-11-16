//! An item that withholds the impl is reported together with the errors
//! that stand beside the impl, an option naming nothing in the trait and a
//! route through no variant, so that none waits for the next build.
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

#[whichever::dispatch(Store, Nope = u8, Item = 1 + 2, via(Wide = .0))]
enum Kind {
    Narrow(Narrow),
}

fn main() {}
