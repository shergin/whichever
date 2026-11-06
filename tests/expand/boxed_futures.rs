//! A boxed future, as `#[async_trait]` writes it, is one type for every
//! arm and is forwarded as it is; when its output is converted, the arms
//! are awaited in one `async move` block, boxed and pinned again with the
//! `Box` the signature names, as the attribute itself writes.
use std::future::Future;
use std::pin::Pin;

#[whichever::dispatchable]
pub trait Store {
    type Error;
    fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<u32, Self::Error>> + Send + 'a>>;
    fn ping<'a>(&'a self) -> Pin<Box<dyn Future<Output = u32> + Send + 'a>>;
}

pub struct Memory;
pub struct Disk;

#[derive(Debug)]
pub struct Unified;

impl From<u8> for Unified {
    fn from(_: u8) -> Self {
        Unified
    }
}

impl From<u16> for Unified {
    fn from(_: u16) -> Self {
        Unified
    }
}

impl Store for Memory {
    type Error = u8;
    fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<u32, u8>> + Send + 'a>> {
        Box::pin(async { Ok(1) })
    }
    fn ping<'a>(&'a self) -> Pin<Box<dyn Future<Output = u32> + Send + 'a>> {
        Box::pin(async { 1 })
    }
}

impl Store for Disk {
    type Error = u16;
    fn load<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<u32, u16>> + Send + 'a>> {
        Box::pin(async { Err(2) })
    }
    fn ping<'a>(&'a self) -> Pin<Box<dyn Future<Output = u32> + Send + 'a>> {
        Box::pin(async { 2 })
    }
}

#[whichever::dispatch(Store, Error = Unified)]
pub enum Any {
    Memory(Memory),
    Disk(Disk),
}

fn main() {}
