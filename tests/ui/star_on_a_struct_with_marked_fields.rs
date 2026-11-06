//! A struct whose marked fields each name their traits marks none bare, so
//! `*` on an inherent method names no field.
#[whichever::dispatchable]
pub trait Rom {
    fn read(&self) -> u8;
}

#[whichever::dispatchable]
pub trait Interrupt {
    fn pending(&self) -> bool;
}

pub struct Cartridge;

impl Rom for Cartridge {
    fn read(&self) -> u8 {
        1
    }
}

impl Cartridge {
    pub fn len(&self) -> usize {
        1
    }
}

pub struct Signals;

impl Interrupt for Signals {
    fn pending(&self) -> bool {
        true
    }
}

impl Signals {
    pub fn len(&self) -> usize {
        2
    }
}

#[whichever::dispatch(Rom)]
#[whichever::dispatch(Interrupt)]
pub struct Mapper {
    #[dispatch(Rom)]
    rom: Cartridge,
    #[dispatch(Interrupt)]
    signals: Signals,
}

#[whichever::dispatch]
impl Mapper {
    #[dispatch(*)]
    pub fn len(&self) -> usize;
}

fn main() {}
