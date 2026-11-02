//! An associated type is fixed with an option, not inside the trait's path.
#[whichever::dispatchable]
trait Storage {
    type Error;
    fn load(&self) -> Result<u32, Self::Error>;
}

struct Memory;

impl Storage for Memory {
    type Error = ();
    fn load(&self) -> Result<u32, ()> {
        Ok(1)
    }
}

#[whichever::dispatch(Storage<Error = ()>)]
enum Kind {
    Memory(Memory),
}

fn main() {}
