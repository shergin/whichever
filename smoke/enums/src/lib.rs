//! Dispatch enums over the traits of `smoke-traits`, across the crate
//! boundary: a `&Self` argument with a fallback, an associated type unified
//! through `Into`, a generic trait at a concrete instantiation, two traits
//! on one enum, `async` methods and `-> Self`.
#![allow(async_fn_in_trait)]
use smoke_traits::{Description, Device, Encoder, Format, Shape, Storage};
use whichever::{dispatch, dispatchable};

#[derive(Debug, Clone)]
pub struct Square(pub u64);

#[derive(Debug, Clone)]
pub struct Circle(pub u64);

impl Shape for Square {
    fn area(&self) -> u64 {
        self.0 * self.0
    }
    fn name(&self) -> &'static str {
        "square"
    }
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn scaled(&self, factor: u64) -> Self {
        Square(self.0 * factor)
    }
}

impl Shape for Circle {
    fn area(&self) -> u64 {
        3 * self.0 * self.0
    }
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn scaled(&self, factor: u64) -> Self {
        Circle(self.0 * factor)
    }
    fn describe(&self) -> Description {
        Description("a circle")
    }
}

#[derive(Debug, Clone)]
#[dispatch(Shape)]
pub enum AnyShape {
    Square(Square),
    Circle(Circle),
}

#[derive(Debug, PartialEq)]
pub struct StorageError(pub String);

#[derive(Debug)]
pub struct DiskError;

impl From<DiskError> for StorageError {
    fn from(_: DiskError) -> Self {
        StorageError("disk".to_owned())
    }
}

pub struct Memory(pub u32);
pub struct Disk;

impl Storage for Memory {
    type Error = StorageError;
    fn load(&self, key: &str) -> Result<u32, StorageError> {
        if key == "answer" {
            return Ok(self.0);
        }
        Err(StorageError(format!("no {key}")))
    }
    async fn flush(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

impl Storage for Disk {
    type Error = DiskError;
    fn load(&self, _key: &str) -> Result<u32, DiskError> {
        Err(DiskError)
    }
    async fn flush(&mut self) -> Result<(), DiskError> {
        Ok(())
    }
}

#[dispatch(Storage, Error = StorageError)]
pub enum AnyStorage {
    Memory(Memory),
    Disk(Disk),
}

pub struct Json;

impl Format for Json {
    type Value = u32;
}

pub struct Compact;
pub struct Pretty;

impl Encoder<Json> for Compact {
    type Output = String;
    fn encode(&self, value: &u32) -> String {
        value.to_string()
    }
}

impl Encoder<Json> for Pretty {
    type Output = String;
    fn encode(&self, value: &u32) -> String {
        format!("{value} ")
    }
    fn encode_all(&self, values: &[u32]) -> Vec<String> {
        vec![format!("{values:?}")]
    }
}

#[dispatch(Encoder<Json>)]
pub enum AnyEncoder {
    Compact(Compact),
    Pretty(Pretty),
}

/// A local trait dispatched on the same enum as the foreign `Device`.
#[dispatchable]
pub trait Peripheral: Device {
    fn ports(&self) -> Vec<u8>;
}

#[derive(Debug, Clone)]
pub struct Printer(pub u32);

impl Printer {
    /// Same name as the trait method: a method-call forward would pick it.
    pub fn kind(&self) -> &'static str {
        "inherent"
    }
}

impl Device for Printer {
    fn id(&self) -> &u32 {
        &self.0
    }
    fn with_id(&self, id: u32) -> Self {
        Printer(id)
    }
    fn kind(&self) -> &'static str {
        "printer"
    }
}

impl Peripheral for Printer {
    fn ports(&self) -> Vec<u8> {
        vec![1]
    }
}

#[derive(Debug, Clone)]
pub struct Scanner(pub u32);

impl Device for Scanner {
    fn id(&self) -> &u32 {
        &self.0
    }
    fn with_id(&self, id: u32) -> Self {
        Scanner(id)
    }
    async fn connect(&mut self) -> Result<(), String> {
        self.0 += 1;
        Ok(())
    }
}

impl Peripheral for Scanner {
    fn ports(&self) -> Vec<u8> {
        vec![]
    }
}

#[derive(Debug, Clone)]
#[whichever::dispatch(Device)]
#[whichever::dispatch(Peripheral)]
pub enum AnyDevice {
    Printer(Printer),
    Scanner(Scanner),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_forward_defaults_and_match_the_same_variant() {
        let square: AnyShape = Square(3).into();
        let circle: AnyShape = Circle(1).into();
        assert_eq!((square.area(), square.name()), (9, "square"));
        assert_eq!((circle.area(), circle.name()), (3, "shape"));
        assert!(square.congruent(&Square(3).into()));
        assert!(!square.congruent(&Square(4).into()));
        assert!(!square.congruent(&circle));
        assert!(matches!(square.scaled(2), AnyShape::Square(Square(6))));
        assert_eq!(circle.describe(), Description("a circle"));
        assert_eq!(square.describe(), Description("a shape"));
    }

    #[test]
    fn storage_errors_convert_into_the_named_type() {
        let disk: AnyStorage = Disk.into();
        assert_eq!(disk.load("answer"), Err(StorageError("disk".to_owned())));
        let mut memory: AnyStorage = Memory(42).into();
        assert_eq!(memory.load("answer"), Ok(42));
        assert!(pollster::block_on(memory.flush()).is_ok());
    }

    #[test]
    fn encoders_dispatch_a_generic_trait() {
        let pretty: AnyEncoder = Pretty.into();
        assert_eq!(pretty.encode(&7), "7 ");
        assert_eq!(pretty.encode_all(&[1, 2]), vec!["[1, 2]".to_owned()]);
        assert_eq!(
            AnyEncoder::from(Compact).encode_all(&[1, 2]),
            vec!["1".to_owned(), "2".to_owned()]
        );
    }

    #[test]
    fn devices_implement_two_traits() {
        let printer: AnyDevice = Printer(1).into();
        assert_eq!(Device::kind(&printer), "printer");
        assert_eq!(printer.kind(), "printer");
        assert!(matches!(printer.with_id(5), AnyDevice::Printer(Printer(5))));
        assert_eq!(printer.apply(|id| id + 1), 2);
        assert_eq!(printer.ports(), vec![1]);
        let mut scanner: AnyDevice = Scanner(1).into();
        pollster::block_on(scanner.connect()).unwrap();
        assert_eq!(*scanner.id(), 2);
        assert_eq!(scanner.kind(), "device");
        assert_eq!(<AnyDevice as Device>::kinds(), &["device"]);
    }
}
