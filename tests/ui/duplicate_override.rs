//! An option given twice is reported on the second.
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

#[whichever::dispatch(Storage, Error = (), Error = u8)]
enum Kind {
    Memory(Memory),
}

fn main() {}
