//! A trait a field names is one the struct dispatches.
#[whichever::dispatchable]
trait Rom {
    fn read(&self) -> u8;
}

#[whichever::dispatchable]
trait Interrupt {
    fn pending(&self) -> bool;
}

struct Cartridge(u8);

impl Rom for Cartridge {
    fn read(&self) -> u8 {
        self.0
    }
}

#[whichever::dispatch(Rom)]
struct Mapper {
    #[dispatch(Interrupt)]
    rom: Cartridge,
}

fn main() {}
