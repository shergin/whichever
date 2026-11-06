//! A struct's field names the traits it is the target of; the field marked
//! bare takes every trait not named elsewhere.
pub trait Rom {
    fn read(&self, address: u16) -> u8;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Rom_b95ec6e492a41184_3_1 as Rom;
pub trait Interrupt {
    fn pending(&self) -> bool;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Interrupt_57de6ee0e4d50e19_8_1 as Interrupt;
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
pub struct Mapper {
    rom: Cartridge,
    signals: Signals,
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Mapper_8f7ff6a481bda9cf_28_1 as Mapper;
#[automatically_derived]
impl Rom for Mapper {
    #[inline]
    fn read(&self, address: u16) -> u8 {
        match self {
            Mapper { rom: __inner, .. } => <Cartridge as Rom>::read(__inner, address),
        }
    }
}
#[automatically_derived]
impl Interrupt for Mapper {
    #[inline]
    fn pending(&self) -> bool {
        match self {
            Mapper { signals: __inner, .. } => <Signals as Interrupt>::pending(__inner),
        }
    }
}
fn main() {}
