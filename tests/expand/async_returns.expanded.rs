use std::future::Future;
pub trait Store {
    async fn flush(&mut self) -> Result<(), ()>;
    fn truncate(&mut self, max: u64) -> impl Future<Output = Result<(), ()>> + Send;
    fn copied(&self) -> impl Future<Output = Self> + Send
    where
        Self: Sized;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Store_78be65bbc36a2a11_3_1 as Store;
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
pub enum AnyStore {
    Memory(Memory),
    Disk(Disk),
}
#[automatically_derived]
impl ::core::convert::From<Memory> for AnyStore {
    #[inline]
    fn from(__value: Memory) -> Self {
        Self::Memory(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Disk> for AnyStore {
    #[inline]
    fn from(__value: Disk) -> Self {
        Self::Disk(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_enum_AnyStore_bc7134310d176a55_51_1 as AnyStore;
#[automatically_derived]
impl Store for AnyStore {
    #[inline]
    async fn flush(&mut self) -> Result<(), ()> {
        match self {
            AnyStore::Memory(__inner) => <Memory as Store>::flush(__inner).await,
            AnyStore::Disk(__inner) => <Disk as Store>::flush(__inner).await,
        }
    }
    #[inline]
    fn truncate(
        &mut self,
        __argument0: u64,
    ) -> impl Future<Output = Result<(), ()>> + Send {
        async move {
            match self {
                AnyStore::Memory(__inner) => {
                    <Memory as Store>::truncate(__inner, __argument0).await
                }
                AnyStore::Disk(__inner) => {
                    <Disk as Store>::truncate(__inner, __argument0).await
                }
            }
        }
    }
    #[inline]
    fn copied(&self) -> impl Future<Output = Self> + Send
    where
        Self: Sized,
    {
        async move {
            match self {
                AnyStore::Memory(__inner) => {
                    AnyStore::Memory(<Memory as Store>::copied(__inner).await)
                }
                AnyStore::Disk(__inner) => {
                    AnyStore::Disk(<Disk as Store>::copied(__inner).await)
                }
            }
        }
    }
}
impl AnyStore {
    pub fn size(&self) -> impl Future<Output = u64> + Send + '_ {
        async move {
            match self {
                Self::Memory(__inner) => __inner.size().await,
                Self::Disk(__inner) => __inner.size().await,
            }
        }
    }
}
fn main() {}
