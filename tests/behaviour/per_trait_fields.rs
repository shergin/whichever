//! A struct's field may name the traits it is the target of; the field
//! marked bare takes every trait not named elsewhere.

#[whichever::dispatchable]
pub trait Rom {
    fn read(&self, address: u16) -> u8;
}

#[whichever::dispatchable]
pub trait Interrupt {
    fn pending(&self) -> bool;
}

#[whichever::dispatchable]
pub trait Named {
    fn name(&self) -> &'static str;
}

pub struct Cartridge(pub Vec<u8>);
pub struct Signals(pub bool);

impl Rom for Cartridge {
    fn read(&self, address: u16) -> u8 {
        self.0[usize::from(address)]
    }
}

impl Named for Cartridge {
    fn name(&self) -> &'static str {
        "cartridge"
    }
}

impl Interrupt for Signals {
    fn pending(&self) -> bool {
        self.0
    }
}

#[whichever::dispatch(Rom)]
#[whichever::dispatch(Interrupt)]
#[whichever::dispatch(Named)]
pub struct Mapper {
    #[dispatch(Rom)]
    rom: Cartridge,
    #[dispatch(Interrupt)]
    signals: Signals,
    #[dispatch]
    label: Cartridge,
}

#[test]
fn each_trait_reaches_its_field() {
    let mapper = Mapper {
        rom: Cartridge(vec![7, 8]),
        signals: Signals(true),
        label: Cartridge(vec![]),
    };
    assert_eq!(mapper.read(1), 8);
    assert!(mapper.pending());
    assert_eq!(mapper.name(), "cartridge");
}
