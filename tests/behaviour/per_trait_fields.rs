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

/// A field names a trait as the struct's attribute spells it, arguments
/// included, so two instantiations of one generic trait, and two traits of
/// one name from two modules, go to two fields.
#[whichever::dispatchable]
pub trait Encoder<F> {
    fn encode(&self, format: F) -> &'static str;
}

pub struct Json;
pub struct Xml;
pub struct JsonEncoder;
pub struct XmlEncoder;

impl Encoder<Json> for JsonEncoder {
    fn encode(&self, _: Json) -> &'static str {
        "json"
    }
}

impl Encoder<Xml> for XmlEncoder {
    fn encode(&self, _: Xml) -> &'static str {
        "xml"
    }
}

pub mod wire {
    #[whichever::dispatchable]
    pub trait Named {
        fn name(&self) -> &'static str;
    }
}

pub struct Wire;

impl wire::Named for Wire {
    fn name(&self) -> &'static str {
        "wire"
    }
}

#[whichever::dispatch(Encoder<Json>)]
#[whichever::dispatch(Encoder<Xml>)]
#[whichever::dispatch(Named)]
#[whichever::dispatch(wire::Named)]
pub struct Codec {
    #[dispatch(Encoder<Json>)]
    json: JsonEncoder,
    #[dispatch(Encoder<Xml>)]
    xml: XmlEncoder,
    #[dispatch(Named)]
    label: Cartridge,
    #[dispatch(wire::Named)]
    wire: Wire,
}

#[test]
fn a_trait_is_the_path_as_the_struct_spells_it() {
    let codec = Codec {
        json: JsonEncoder,
        xml: XmlEncoder,
        label: Cartridge(vec![]),
        wire: Wire,
    };
    assert_eq!(Encoder::<Json>::encode(&codec, Json), "json");
    assert_eq!(Encoder::<Xml>::encode(&codec, Xml), "xml");
    assert_eq!(Named::name(&codec), "cartridge");
    assert_eq!(wire::Named::name(&codec), "wire");
}
