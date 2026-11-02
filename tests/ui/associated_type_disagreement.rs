#[whichever::dispatchable]
trait Store {
    type Item;
    fn first(&self) -> Self::Item;
}

struct Narrow;
struct Wide;

impl Store for Narrow {
    type Item = u32;
    fn first(&self) -> u32 {
        1
    }
}

impl Store for Wide {
    type Item = u64;
    fn first(&self) -> u64 {
        1
    }
}

#[whichever::dispatch(Store)]
enum Kind {
    Narrow(Narrow),
    Wide(Wide),
}

fn main() {}
