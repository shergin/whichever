use std::future::Future;

#[whichever::dispatchable]
pub trait Store {
    async fn flush(&mut self) -> Result<(), ()>;
    fn truncate(&mut self, max: u64) -> impl Future<Output = Result<(), ()>> + Send;
    fn copied(&self) -> impl Future<Output = Self> + Send
    where
        Self: Sized;
}

pub struct Memory;
pub struct Disk;

impl Store for Memory {
    async fn flush(&mut self) -> Result<(), ()> {
        Ok(())
    }
    fn truncate(&mut self, _max: u64) -> impl Future<Output = Result<(), ()>> + Send {
        async { Ok(()) }
    }
    fn copied(&self) -> impl Future<Output = Self> + Send {
        async { Memory }
    }
}

impl Store for Disk {
    async fn flush(&mut self) -> Result<(), ()> {
        Err(())
    }
    fn truncate(&mut self, _max: u64) -> impl Future<Output = Result<(), ()>> + Send {
        async { Err(()) }
    }
    fn copied(&self) -> impl Future<Output = Self> + Send {
        async { Disk }
    }
}

impl Memory {
    pub fn size(&self) -> impl Future<Output = u64> + Send + '_ {
        async { 1 }
    }
}

impl Disk {
    pub fn size(&self) -> impl Future<Output = u64> + Send + '_ {
        async { 2 }
    }
}

#[whichever::dispatch(Store)]
pub enum AnyStore {
    Memory(Memory),
    Disk(Disk),
}

#[whichever::dispatch]
impl AnyStore {
    #[dispatch(*)]
    pub fn size(&self) -> impl Future<Output = u64> + Send + '_;
}

fn main() {}
