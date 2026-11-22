//! A trait named on two fields is one error, and the impl is still
//! written, so the call below finds it.
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

fn read(mapper: &Mapper) -> u8 {
    mapper.read()
}

fn main() {
    let _ = read;
}
