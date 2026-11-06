//! A trait is the target of one field.
#[whichever::dispatchable]
trait Rom {
    fn read(&self) -> u8;
}

struct Cartridge(u8);

impl Rom for Cartridge {
    fn read(&self) -> u8 {
        self.0
    }
}

#[whichever::dispatch(Rom)]
struct Mapper {
    #[dispatch(Rom)]
    rom: Cartridge,
    #[dispatch(Rom)]
    other: Cartridge,
}

fn main() {}
