//! The desugared `-> impl Future` spelling is the point of this module.
#![allow(clippy::manual_async_fn, clippy::needless_lifetimes)]

use std::future::Future;

#[whichever::dispatchable]
trait Store {
    fn truncate(&mut self, max: u64) -> impl Future<Output = Result<(), ()>> + Send;
    fn consume(self, tag: &str) -> impl Future<Output = usize> + Send
    where
        Self: Sized;
    fn first<'a, 'b>(&'a self, s: &'b str) -> impl Future<Output = (&'a u8, usize)> + Send;
    fn copied(&self) -> impl Future<Output = Self> + Send
    where
        Self: Sized;
}

#[derive(Debug, PartialEq)]
struct Memory(u8);
#[derive(Debug, PartialEq)]
struct Disk(u8);

impl Store for Memory {
    fn truncate(&mut self, max: u64) -> impl Future<Output = Result<(), ()>> + Send {
        self.0 = self.0.min(max as u8);
        async { Ok(()) }
    }
    fn consume(self, tag: &str) -> impl Future<Output = usize> + Send {
        async move { tag.len() + usize::from(self.0) }
    }
    fn first<'a, 'b>(&'a self, s: &'b str) -> impl Future<Output = (&'a u8, usize)> + Send {
        async move { (&self.0, s.len()) }
    }
    fn copied(&self) -> impl Future<Output = Self> + Send {
        async move { Memory(self.0) }
    }
}

impl Store for Disk {
    fn truncate(&mut self, _max: u64) -> impl Future<Output = Result<(), ()>> + Send {
        async { Err(()) }
    }
    fn consume(self, tag: &str) -> impl Future<Output = usize> + Send {
        async move { tag.len() * usize::from(self.0) }
    }
    fn first<'a, 'b>(&'a self, s: &'b str) -> impl Future<Output = (&'a u8, usize)> + Send {
        async move { (&self.0, s.len()) }
    }
    fn copied(&self) -> impl Future<Output = Self> + Send {
        async move { Disk(self.0) }
    }
}

impl Memory {
    fn fetch(&self) -> impl Future<Output = u8> + Send + '_ {
        async move { self.0 }
    }
}

impl Disk {
    fn fetch(&self) -> impl Future<Output = u8> + Send + '_ {
        async move { self.0 * 10 }
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(Store)]
enum AnyStore {
    Memory(Memory),
    Disk(Disk),
}

#[whichever::dispatch]
impl AnyStore {
    #[dispatch(*)]
    fn fetch(&self) -> impl Future<Output = u8> + Send + '_;
}

/// Compiles only when the enum's futures are `Send`.
fn spawnable<T: Store + Sync>(store: &mut T) -> impl Future<Output = Result<(), ()>> + Send + '_ {
    store.truncate(1)
}

#[test]
fn every_arm_is_awaited_inside_one_future() {
    let mut memory: AnyStore = Memory(9).into();
    assert_eq!(pollster::block_on(spawnable(&mut memory)), Ok(()));
    assert_eq!(memory, AnyStore::Memory(Memory(1)));
    let mut disk: AnyStore = Disk(2).into();
    assert_eq!(pollster::block_on(disk.truncate(1)), Err(()));
    assert_eq!(pollster::block_on(disk.first("ab")), (&2, 2));
    assert_eq!(pollster::block_on(disk.copied()), AnyStore::Disk(Disk(2)));
    assert_eq!(pollster::block_on(disk.fetch()), 20);
    assert_eq!(pollster::block_on(disk.consume("abc")), 6);
}
