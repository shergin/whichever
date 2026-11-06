//! `#[async_trait]` goes above `#[dispatchable]`: the carrier then holds the
//! signatures as `async_trait` rewrote them, which is what every impl has.

use async_trait::async_trait;

#[async_trait]
#[whichever::dispatchable]
pub trait Fetch {
    async fn fetch(&self) -> u32;
    fn sync(&self) -> u32 {
        0
    }
}

pub struct Fast;
pub struct Slow;

#[async_trait]
impl Fetch for Fast {
    async fn fetch(&self) -> u32 {
        1
    }
}

#[async_trait]
impl Fetch for Slow {
    async fn fetch(&self) -> u32 {
        2
    }
    fn sync(&self) -> u32 {
        2
    }
}

#[whichever::dispatch(Fetch)]
pub enum Any {
    Fast(Fast),
    Slow(Slow),
}

#[test]
fn the_boxed_future_is_forwarded_as_it_is() {
    let slow: Any = Slow.into();
    assert_eq!(pollster::block_on(slow.fetch()), 2);
    assert_eq!(slow.sync(), 2);
    assert_eq!(pollster::block_on(Any::from(Fast).fetch()), 1);
}

/// An overridden associated type under `#[async_trait]`: the boxed future
/// is awaited in one `async move` block, converted, and boxed and pinned
/// again, as the attribute itself writes an `async fn` of the impl.
#[async_trait]
#[whichever::dispatchable]
pub trait Store {
    type Error;
    async fn load(&self) -> Result<u32, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub struct Unified(pub &'static str);

impl From<u8> for Unified {
    fn from(code: u8) -> Self {
        Unified(if code == 0 { "memory" } else { "memory error" })
    }
}

impl From<&'static str> for Unified {
    fn from(message: &'static str) -> Self {
        Unified(message)
    }
}

pub struct Memory;
pub struct Disk;

#[async_trait]
impl Store for Memory {
    type Error = u8;
    async fn load(&self) -> Result<u32, u8> {
        Ok(7)
    }
}

#[async_trait]
impl Store for Disk {
    type Error = &'static str;
    async fn load(&self) -> Result<u32, &'static str> {
        Err("offline")
    }
}

#[whichever::dispatch(Store, Error = Unified)]
pub enum AnyStore {
    Memory(Memory),
    Disk(Disk),
}

#[test]
fn the_boxed_futures_output_is_converted() {
    let memory = AnyStore::Memory(Memory);
    assert_eq!(pollster::block_on(memory.load()), Ok(7));
    let disk = AnyStore::Disk(Disk);
    assert_eq!(pollster::block_on(disk.load()), Err(Unified("offline")));
}
