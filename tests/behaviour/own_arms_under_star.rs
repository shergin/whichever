//! Under `#[dispatch(*)]` a method's body holds the arms the enum answers
//! itself, and every variant those arms do not name is forwarded: a
//! forwarded variant answers what its payload answers, and an answered one
//! what its arm says, even where its payload has the impl. The enums model
//! the surveyed shapes, mp4-atom's `Codec` and `DrefEntry`, hickory-proto's
//! `RData` and PathMap's `TaggedNodeRef`, over local stand-in types.
#![deny(deprecated)]

use std::fmt::{self, Formatter};

/// A four-byte code, as mp4-atom's `FourCC`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FourCc(pub [u8; 4]);

#[derive(Debug, PartialEq, Eq)]
pub enum EncodeError {
    UnknownCodec,
    Payload(&'static str),
}

#[whichever::dispatchable]
pub trait Encode {
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError>;
}

pub struct Avc1(pub u8);
pub struct Hev1(pub u8);

impl Encode for Avc1 {
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
        buf.extend_from_slice(b"avc1");
        buf.push(self.0);
        Ok(())
    }
}

impl Encode for Hev1 {
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
        buf.extend_from_slice(b"hev1");
        buf.push(self.0);
        Ok(())
    }
}

#[whichever::dispatch]
pub enum Codec {
    Avc1(Avc1),
    Hev1(Hev1),
    Unknown(FourCc, Vec<u8>),
}

// mp4-atom 0.17.1's `Codec`: every codec is forwarded, and the unknown one
// answers `Err`.
#[whichever::dispatch]
impl Encode for Codec {
    #[dispatch(*)]
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
        match self {
            Self::Unknown(..) => Err(EncodeError::UnknownCodec),
        }
    }
}

pub struct Url(pub &'static str);
pub struct Urn(pub &'static str);

impl Encode for Url {
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
        buf.extend_from_slice(b"url ");
        buf.extend_from_slice(self.0.as_bytes());
        Ok(())
    }
}

impl Encode for Urn {
    fn encode(&self, _buf: &mut Vec<u8>) -> Result<(), EncodeError> {
        Err(EncodeError::Payload("urn"))
    }
}

#[whichever::dispatch]
pub enum DrefEntry {
    Url(Url),
    Urn(Urn),
    Unknown(FourCc, Vec<u8>),
}

// mp4-atom's `DrefEntry`: the unknown entry's arm encodes both of its
// fields.
#[whichever::dispatch]
impl Encode for DrefEntry {
    #[dispatch(*)]
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
        match self {
            Self::Unknown(kind, data) => {
                buf.extend_from_slice(&kind.0);
                buf.extend_from_slice(data);
                Ok(())
            }
        }
    }
}

#[test]
fn a_codec_forwards_and_the_unknown_one_answers_its_arm() {
    let mut buf = vec![];
    assert_eq!(Codec::from(Avc1(7)).encode(&mut buf), Ok(()));
    assert_eq!(Codec::from(Hev1(8)).encode(&mut buf), Ok(()));
    assert_eq!(buf, b"avc1\x07hev1\x08");
    let unknown = Codec::Unknown(FourCc(*b"mp4v"), vec![1, 2]);
    assert_eq!(unknown.encode(&mut buf), Err(EncodeError::UnknownCodec));
    assert_eq!(buf, b"avc1\x07hev1\x08", "the arm writes nothing");
}

#[test]
fn a_dref_entry_arm_reads_both_fields_of_its_variant() {
    let mut buf = vec![];
    assert_eq!(DrefEntry::from(Url("x")).encode(&mut buf), Ok(()));
    assert_eq!(
        DrefEntry::from(Urn("y")).encode(&mut buf),
        Err(EncodeError::Payload("urn")),
        "a forwarded variant answers what its payload answers"
    );
    let unknown = DrefEntry::Unknown(FourCc(*b"alis"), vec![9, 9]);
    assert_eq!(unknown.encode(&mut buf), Ok(()));
    assert_eq!(buf, b"url xalis\x09\x09");
}

#[whichever::dispatchable]
pub trait BinEncodable {
    fn emit(&self, encoder: &mut Vec<u8>) -> Result<(), EncodeError>;
}

pub struct A(pub [u8; 4]);
pub struct Opt(pub u16);
/// The record type an update names, which has an encoding of its own the
/// enum does not use.
pub struct RecordType(pub u16);

impl BinEncodable for A {
    fn emit(&self, encoder: &mut Vec<u8>) -> Result<(), EncodeError> {
        encoder.extend_from_slice(&self.0);
        Ok(())
    }
}

impl BinEncodable for Opt {
    fn emit(&self, encoder: &mut Vec<u8>) -> Result<(), EncodeError> {
        encoder.extend_from_slice(&self.0.to_be_bytes());
        Ok(())
    }
}

impl BinEncodable for RecordType {
    fn emit(&self, encoder: &mut Vec<u8>) -> Result<(), EncodeError> {
        encoder.extend_from_slice(&self.0.to_be_bytes());
        Ok(())
    }
}

impl fmt::Display for A {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let [a, b, c, d] = self.0;
        write!(f, "{a}.{b}.{c}.{d}")
    }
}

impl fmt::Display for Opt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "OPT {}", self.0)
    }
}

impl fmt::Display for RecordType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "TYPE{}", self.0)
    }
}

#[whichever::dispatch]
pub enum RData {
    A(A),
    Opt(Opt),
    Update0(RecordType),
    Unknown { code: u16, rdata: Vec<u8> },
    Zero,
}

// hickory-proto 0.26.3's `RData`: `Opt` and `Update0` each hold a payload
// that has both impls, and the enum answers for `Update0` in `emit` and
// for both in `Display` itself.
#[whichever::dispatch]
impl BinEncodable for RData {
    #[dispatch(*)]
    fn emit(&self, encoder: &mut Vec<u8>) -> Result<(), EncodeError> {
        match self {
            Self::Update0(_) => Ok(()),
            Self::Unknown { rdata, .. } => {
                encoder.extend_from_slice(rdata);
                Ok(())
            }
            Self::Zero => Ok(()),
        }
    }
}

#[whichever::dispatch]
impl whichever::foreign::Display for RData {
    #[dispatch(*)]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Opt(_) => Err(fmt::Error),
            Self::Update0(_) => f.write_str("UPDATE"),
            Self::Unknown { code, .. } => write!(f, "TYPE{code} unknown"),
            Self::Zero => f.write_str(""),
        }
    }
}

#[test]
fn rdata_forwards_and_answers_for_variants_whose_payload_has_the_impl() {
    let mut encoder = vec![];
    assert_eq!(RData::from(A([1, 2, 3, 4])).emit(&mut encoder), Ok(()));
    assert_eq!(RData::from(Opt(0x0102)).emit(&mut encoder), Ok(()));
    assert_eq!(RData::from(RecordType(0x0909)).emit(&mut encoder), Ok(()));
    let unknown = RData::Unknown {
        code: 99,
        rdata: vec![7],
    };
    assert_eq!(unknown.emit(&mut encoder), Ok(()));
    assert_eq!(RData::Zero.emit(&mut encoder), Ok(()));
    assert_eq!(
        encoder,
        [1, 2, 3, 4, 1, 2, 7],
        "`Update0` answers its arm and writes nothing, as `Zero` does"
    );
    assert_eq!(RData::from(A([10, 0, 0, 1])).to_string(), "10.0.0.1");
    assert_eq!(RData::from(RecordType(5)).to_string(), "UPDATE");
    assert_eq!(unknown.to_string(), "TYPE99 unknown");
    assert_eq!(RData::Zero.to_string(), "");
    let mut written = String::new();
    assert!(
        fmt::write(&mut written, format_args!("{}", RData::from(Opt(1)))).is_err(),
        "`Opt` answers `Err` although `Opt` has `Display`"
    );
}

pub struct DenseNode(pub Vec<u32>);
pub struct LineNode(pub u32);

impl DenseNode {
    pub fn count(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, key: usize) -> Option<u32> {
        self.0.get(key).copied()
    }
    pub fn push(&mut self, value: u32) -> usize {
        self.0.push(value);
        self.0.len()
    }
    pub fn into_values(self) -> Vec<u32> {
        self.0
    }
    pub fn describe(&self) -> String {
        format!("dense of {}", self.0.len())
    }
    pub async fn load(&self) -> u32 {
        self.0.iter().sum()
    }
}

impl LineNode {
    pub fn count(&self) -> usize {
        1
    }
    pub fn get(&self, key: usize) -> Option<u32> {
        (key == 0).then_some(self.0)
    }
    pub fn push(&mut self, value: u32) -> usize {
        self.0 = value;
        1
    }
    pub fn into_values(self) -> Vec<u32> {
        vec![self.0]
    }
    pub fn describe(&self) -> String {
        format!("line of {}", self.0)
    }
    pub async fn load(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
pub enum NodeRef {
    Dense(DenseNode),
    Line(LineNode),
    Empty,
}

// PathMap's `TaggedNodeRef`: inherent methods beside a unit `EmptyNode`,
// under each receiver, `call =`, `with =` and `async`.
#[whichever::dispatch]
impl NodeRef {
    #[dispatch(*)]
    pub fn count(&self) -> usize {
        match self {
            Self::Empty => 0,
        }
    }

    #[dispatch(*)]
    pub fn get(&self, key: usize) -> Option<u32> {
        match self {
            Self::Empty => None,
        }
    }

    #[dispatch(*)]
    pub fn push(&mut self, value: u32) -> usize {
        match self {
            Self::Empty => {
                *self = Self::Line(LineNode(value));
                1
            }
        }
    }

    #[dispatch(*)]
    pub fn into_values(self) -> Vec<u32> {
        match self {
            Self::Empty => vec![],
        }
    }

    #[dispatch(*, call = describe)]
    pub fn label(&self) -> String {
        match self {
            Self::Empty => "empty".to_owned(),
        }
    }

    #[dispatch(*, call = count, with = Some)]
    pub fn known_count(&self) -> Option<usize> {
        match self {
            Self::Empty => None,
        }
    }

    #[dispatch(*)]
    pub async fn load(&self) -> u32 {
        match self {
            Self::Empty => u32::MAX,
        }
    }
}

#[test]
fn node_methods_forward_under_every_receiver_and_the_empty_node_answers_its_arm() {
    let dense = NodeRef::from(DenseNode(vec![4, 5, 6]));
    let line = NodeRef::from(LineNode(9));
    assert_eq!(
        (dense.count(), line.count(), NodeRef::Empty.count()),
        (3, 1, 0)
    );
    assert_eq!(
        (dense.get(2), line.get(0), NodeRef::Empty.get(0)),
        (Some(6), Some(9), None)
    );
    assert_eq!(dense.label(), "dense of 3");
    assert_eq!(NodeRef::Empty.label(), "empty");
    assert_eq!(line.known_count(), Some(1));
    assert_eq!(
        NodeRef::Empty.known_count(),
        None,
        "an own arm is not wrapped by `with`"
    );
    assert_eq!(pollster::block_on(dense.load()), 15);
    assert_eq!(pollster::block_on(NodeRef::Empty.load()), u32::MAX);
    let mut grown = NodeRef::Empty;
    assert_eq!(grown.push(3), 1);
    assert_eq!(grown.get(0), Some(3), "the arm replaced the empty node");
    assert_eq!(grown.push(4), 1, "the line node answers now");
    let mut dense = dense;
    assert_eq!(dense.push(7), 4);
    assert_eq!(dense.into_values(), [4, 5, 6, 7]);
    assert_eq!(line.into_values(), [9]);
    assert_eq!(NodeRef::Empty.into_values(), Vec::<u32>::new());
}

#[whichever::dispatchable]
pub trait Rename {
    fn name(&self) -> String;
    fn rename(&mut self, name: &str);
    fn into_name(self) -> String;
    async fn fetch(&self) -> u32;
}

pub struct Named(pub String);
pub struct Labelled(pub String);

impl Rename for Named {
    fn name(&self) -> String {
        self.0.clone()
    }
    fn rename(&mut self, name: &str) {
        self.0 = name.to_owned();
    }
    fn into_name(self) -> String {
        self.0
    }
    async fn fetch(&self) -> u32 {
        1
    }
}

impl Rename for Labelled {
    fn name(&self) -> String {
        format!("label {}", self.0)
    }
    fn rename(&mut self, name: &str) {
        self.0 = name.to_uppercase();
    }
    fn into_name(self) -> String {
        self.0
    }
    async fn fetch(&self) -> u32 {
        2
    }
}

#[whichever::dispatch]
pub enum Entity {
    Named(Named),
    Labelled(Labelled),
    #[deprecated(note = "use `Named`")]
    #[dispatch(from = false)]
    Legacy(Named),
    Anonymous,
}

// A trait's methods under each receiver, and an `async fn`; the deprecated
// variant is forwarded, under the allowance a hand writes on its arm.
#[whichever::dispatch]
impl Rename for Entity {
    #[dispatch(*)]
    fn name(&self) -> String {
        match self {
            Self::Anonymous => "anonymous".to_owned(),
        }
    }

    #[dispatch(*)]
    fn rename(&mut self, name: &str) {
        match self {
            Self::Anonymous => *self = Self::Named(Named(name.to_owned())),
        }
    }

    #[dispatch(*)]
    fn into_name(self) -> String {
        match self {
            Self::Anonymous => String::new(),
        }
    }

    #[dispatch(*)]
    async fn fetch(&self) -> u32 {
        match self {
            Self::Anonymous => 0,
        }
    }
}

#[test]
fn a_trait_method_under_each_receiver_forwards_and_answers() {
    let mut labelled = Entity::from(Labelled("a".to_owned()));
    assert_eq!(labelled.name(), "label a");
    labelled.rename("b");
    assert_eq!(labelled.name(), "label B");
    assert_eq!(labelled.into_name(), "B");
    #[allow(deprecated)]
    let legacy = Entity::Legacy(Named("old".to_owned()));
    assert_eq!(legacy.name(), "old");
    let mut anonymous = Entity::Anonymous;
    assert_eq!(anonymous.name(), "anonymous");
    assert_eq!(pollster::block_on(anonymous.fetch()), 0);
    anonymous.rename("c");
    assert_eq!(anonymous.name(), "c", "the arm made it a named entity");
    assert_eq!(pollster::block_on(anonymous.fetch()), 1);
    assert_eq!(
        pollster::block_on(Entity::from(Labelled(String::new())).fetch()),
        2
    );
    assert_eq!(Entity::Anonymous.into_name(), "");
}

#[whichever::dispatchable]
pub trait Weigh {
    fn weight(&self) -> u32;
}

pub struct Light(pub u32);
pub struct Heavy(pub u32);

impl Weigh for Light {
    fn weight(&self) -> u32 {
        self.0
    }
}

impl Weigh for Heavy {
    fn weight(&self) -> u32 {
        self.0 * 10
    }
}

impl Weigh for FourCc {
    fn weight(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }
}

#[whichever::dispatch]
pub enum Load {
    Light(Light),
    Heavy(Heavy),
    Coded(FourCc),
    Unknown(FourCc, u32),
}

// An arm under a `cfg` that holds answers, and its variant is forwarded
// where the `cfg` does not; an integration test is compiled with
// `cfg(test)`. A guarded arm sits beside its unguarded one.
#[whichever::dispatch]
impl Weigh for Load {
    #[dispatch(*)]
    fn weight(&self) -> u32 {
        match self {
            #[cfg(test)]
            Self::Light(_) => 0,
            #[cfg(not(test))]
            Self::Heavy(_) => 0,
            Self::Coded(kind) if kind.0 == *b"none" => 1,
            Self::Coded(_) => 2,
            Self::Unknown(_, weight) => *weight,
        }
    }
}

#[test]
fn a_conditional_arm_answers_where_its_cfg_holds_and_forwards_elsewhere() {
    assert_eq!(Load::from(Light(5)).weight(), 0, "the `cfg` holds");
    assert_eq!(
        Load::from(Heavy(5)).weight(),
        50,
        "the `cfg` does not hold, and `Heavy` is forwarded"
    );
    assert_eq!(Load::from(FourCc(*b"none")).weight(), 1, "the guard holds");
    assert_eq!(
        Load::from(FourCc(*b"mp4a")).weight(),
        2,
        "the unguarded arm answers"
    );
    assert_eq!(Load::Unknown(FourCc(*b"mp4a"), 33).weight(), 33);
}

pub struct Path(pub Vec<u32>);

impl DenseNode {
    pub fn join(&self, other: &Path) -> Vec<u32> {
        self.0.iter().chain(&other.0).copied().collect()
    }
}

impl LineNode {
    pub fn join(&self, other: &Path) -> Vec<u32> {
        let mut joined = vec![self.0];
        joined.extend(&other.0);
        joined
    }
}

// PathMap's `pjoin_dyn`: a statement runs for every variant before the
// `match` over the node kinds. The statement stays in a plain method, and
// the `match` moves into a private method under `*` that calls the
// payloads' `join`.
#[whichever::dispatch]
impl NodeRef {
    pub fn join(&self, other: &Path) -> Vec<u32> {
        let other = Path(other.0.iter().copied().filter(|&step| step != 0).collect());
        self.join_by_kind(&other)
    }

    #[dispatch(*, call = join)]
    fn join_by_kind(&self, other: &Path) -> Vec<u32> {
        match self {
            Self::Empty => other.0.clone(),
        }
    }
}

#[test]
fn a_plain_method_runs_its_statements_and_calls_the_arms_under_star() {
    let other = Path(vec![8]);
    assert_eq!(NodeRef::from(DenseNode(vec![1, 2])).join(&other), [1, 2, 8]);
    assert_eq!(NodeRef::from(LineNode(5)).join(&other), [5, 8]);
    assert_eq!(
        NodeRef::Empty.join(&other),
        [8],
        "the empty node answers its arm"
    );
    assert_eq!(
        NodeRef::from(LineNode(5)).join(&Path(vec![0, 8])),
        [5, 8],
        "the statement before the `match` runs for a forwarded variant"
    );
    assert_eq!(
        NodeRef::Empty.join(&Path(vec![0, 8])),
        [8],
        "and for a variant the enum answers itself"
    );
}
