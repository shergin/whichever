//! A struct's field names the traits it is the target of; the field marked
//! bare takes every trait not named elsewhere.
#[whichever::dispatchable]
pub trait Rom {
    fn read(&self, address: u16) -> u8;
}

#[whichever::dispatchable]
pub trait Interrupt {
    fn pending(&self) -> bool;
}

pub struct Cartridge(pub Vec<u8>);
pub struct Signals(pub bool);

impl Rom for Cartridge {
    fn read(&self, address: u16) -> u8 {
        self.0[usize::from(address)]
    }
}

impl Interrupt for Signals {
    fn pending(&self) -> bool {
        self.0
    }
}

#[whichever::dispatch(Rom)]
#[whichever::dispatch(Interrupt)]
pub struct Mapper {
    #[dispatch(Rom)]
    rom: Cartridge,
    #[dispatch]
    signals: Signals,
}

fn main() {}
