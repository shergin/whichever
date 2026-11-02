//! An override or a mismatch naming nothing in the trait is a typo, and is
//! reported instead of turning into a type error somewhere else.
#[whichever::dispatchable]
trait Storage {
    type Error;
    fn load(&self) -> Result<u32, Self::Error>;
    #[dispatch(mismatch = false)]
    fn same(&self, other: &Self) -> bool;
}

struct Memory;

impl Storage for Memory {
    type Error = ();
    fn load(&self) -> Result<u32, ()> {
        Ok(1)
    }
    fn same(&self, _: &Self) -> bool {
        true
    }
}

#[whichever::dispatch(Storage, Eror = (), mismatch(sane = false))]
enum Kind {
    Memory(Memory),
}

fn main() {}
