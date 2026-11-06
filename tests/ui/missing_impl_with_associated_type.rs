//! A first variant whose payload lacks the impl, on a trait with an
//! associated type: the missing bound is reported on the variant, not on
//! the projection the associated type is taken from.
#[whichever::dispatchable]
trait Storage {
    type Error;
    fn load(&self) -> Result<u32, Self::Error>;
}

struct Memory;
struct Disk;

impl Storage for Disk {
    type Error = u8;
    fn load(&self) -> Result<u32, u8> {
        Ok(1)
    }
}

#[whichever::dispatch(Storage)]
enum Kind {
    Memory(Memory),
    Disk(Disk),
}

fn main() {}
