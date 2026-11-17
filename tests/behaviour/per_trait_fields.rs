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

/// A trait's impl has one home, the struct's attribute or a partial impl
/// block, and a struct forwards it to the field that names it either way.
/// The shape is rsynko 0.1.2's `LawEnv`: a catalog's trait needs a header
/// that ties its `Extractor` to the syntax's, which the attribute cannot
/// write, beside traits the attribute forwards to their fields.
mod partial_blocks {
    #[whichever::dispatchable]
    pub trait Sorts {
        type Extractor;
    }

    #[whichever::dispatchable]
    pub trait Catalog: Sorts {
        fn keys(&self) -> Vec<Self::Extractor>;
        fn size(&self) -> usize {
            self.keys().len()
        }
    }

    #[whichever::dispatchable]
    pub trait Named {
        fn name(&self) -> String;
    }

    #[whichever::dispatchable]
    pub trait Versioned {
        fn version(&self) -> u32;
    }

    pub struct Syntax;

    impl Sorts for Syntax {
        type Extractor = u8;
    }

    pub struct Books(pub Vec<u8>);

    impl Sorts for Books {
        type Extractor = u8;
    }

    impl Catalog for Books {
        fn keys(&self) -> Vec<u8> {
            self.0.clone()
        }
        fn size(&self) -> usize {
            100 + self.0.len()
        }
    }

    impl Versioned for Books {
        fn version(&self) -> u32 {
            3
        }
    }

    pub struct Label(pub &'static str);

    impl Named for Label {
        fn name(&self) -> String {
            self.0.to_owned()
        }
    }

    #[whichever::dispatch(Named)]
    #[whichever::dispatch(Versioned)]
    pub struct Env<S, C, N> {
        pub syntax: S,
        #[dispatch(Catalog, Versioned)]
        pub catalog: C,
        #[dispatch]
        pub named: N,
    }

    impl<S: Sorts, C, N> Sorts for Env<S, C, N> {
        type Extractor = S::Extractor;
    }

    #[whichever::dispatch]
    impl<S: Sorts, C: Catalog<Extractor = S::Extractor>, N> Catalog for Env<S, C, N> {}

    fn env() -> Env<Syntax, Books, Label> {
        Env {
            syntax: Syntax,
            catalog: Books(vec![1, 2]),
            named: Label("env"),
        }
    }

    #[test]
    fn a_blocks_trait_reaches_the_field_that_names_it() {
        let env = env();
        assert_eq!(env.keys(), [1, 2]);
        assert_eq!(env.size(), 102, "the payload's own `size`");
        assert_eq!(env.version(), 3);
        assert_eq!(env.name(), "env");
    }

    /// A block's method may be marked `*`, which is the field the trait goes
    /// to, or name that field.
    mod marked_methods {
        use super::{Books, Catalog, Label, Sorts, Syntax};

        #[whichever::dispatch]
        pub struct Env<S, C, N> {
            pub syntax: S,
            #[dispatch(Catalog)]
            pub catalog: C,
            #[dispatch]
            pub named: N,
        }

        impl<S: Sorts, C, N> Sorts for Env<S, C, N> {
            type Extractor = S::Extractor;
        }

        #[whichever::dispatch]
        impl<S: Sorts, C: Catalog<Extractor = S::Extractor>, N> Catalog for Env<S, C, N> {
            #[dispatch(*)]
            fn keys(&self) -> Vec<Self::Extractor>;
            #[dispatch(catalog)]
            fn size(&self) -> usize;
        }

        #[test]
        fn star_and_the_fields_name_reach_the_same_field() {
            let env = Env {
                syntax: Syntax,
                catalog: Books(vec![4]),
                named: Label("unused"),
            };
            assert_eq!(env.keys(), [4]);
            assert_eq!(env.size(), 101);
        }
    }

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

    impl Named for JsonEncoder {
        fn name(&self) -> String {
            String::from("json encoder")
        }
    }

    /// A field names a generic trait as the block's header spells it, its
    /// parameter included.
    mod generic {
        use super::{Encoder, Json, JsonEncoder, Label, Named};

        #[whichever::dispatch(Named)]
        pub struct Env<E, N> {
            #[dispatch(Encoder<F>)]
            pub encoder: E,
            #[dispatch]
            pub named: N,
        }

        #[whichever::dispatch]
        impl<F, E: Encoder<F>, N> Encoder<F> for Env<E, N> {}

        #[test]
        fn the_trait_at_the_blocks_parameter_reaches_its_field() {
            let env = Env {
                encoder: JsonEncoder,
                named: Label("env"),
            };
            assert_eq!(env.encode(Json), "json");
            assert_eq!(env.name(), "env");
        }
    }

    /// Two instantiations of one trait are two traits: one is the attribute's,
    /// forwarded to the bare field, and the other a block's, forwarded to the
    /// field that names it.
    mod two_instantiations {
        use super::{Encoder, Json, JsonEncoder, Xml, XmlEncoder};

        #[whichever::dispatch(Encoder<Json>)]
        pub struct Codec<X> {
            #[dispatch]
            pub json: JsonEncoder,
            #[dispatch(Encoder<Xml>)]
            pub xml: X,
        }

        #[whichever::dispatch]
        impl<X: Encoder<Xml>> Encoder<Xml> for Codec<X> {}

        #[test]
        fn each_instantiation_reaches_its_field() {
            let codec = Codec {
                json: JsonEncoder,
                xml: XmlEncoder,
            };
            assert_eq!(Encoder::<Json>::encode(&codec, Json), "json");
            assert_eq!(Encoder::<Xml>::encode(&codec, Xml), "xml");
        }
    }

    /// Two traits of one name from two modules are two traits: the attribute's
    /// goes to the bare field, and the block's to the field that names it.
    mod two_paths {
        pub mod a {
            #[whichever::dispatchable]
            pub trait Named {
                fn name(&self) -> &'static str;
            }
        }

        pub mod b {
            #[whichever::dispatchable]
            pub trait Named {
                fn name(&self) -> &'static str;
            }
        }

        pub struct First;
        pub struct Second;

        impl a::Named for First {
            fn name(&self) -> &'static str {
                "first"
            }
        }

        impl b::Named for Second {
            fn name(&self) -> &'static str {
                "second"
            }
        }

        #[whichever::dispatch(a::Named)]
        pub struct Both<S> {
            #[dispatch]
            pub first: First,
            #[dispatch(b::Named)]
            pub second: S,
        }

        #[whichever::dispatch]
        impl<S: b::Named> b::Named for Both<S> {}

        #[test]
        fn each_trait_reaches_its_field() {
            let both = Both {
                first: First,
                second: Second,
            };
            assert_eq!(a::Named::name(&both), "first");
            assert_eq!(b::Named::name(&both), "second");
        }
    }
}
