//! A boxed future, as `#[async_trait]` writes it, is one type for every
//! arm and is forwarded as it is; when its output is converted, the arms
//! are awaited in one `async move` block, boxed and pinned again with the
//! `Box` the signature names, as the attribute itself writes.
use std::future::Future;
use std::pin::Pin;
pub trait Store {
    type Error;
    fn load<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<u32, Self::Error>> + Send + 'a>>;
    fn ping<'a>(&'a self) -> Pin<Box<dyn Future<Output = u32> + Send + 'a>>;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Store_c21bb114c9c8b560_8_1 as Store;
pub struct Memory;
pub struct Disk;
pub struct Unified;
#[automatically_derived]
impl ::core::fmt::Debug for Unified {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "Unified")
    }
}
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
    fn load<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<u32, u16>> + Send + 'a>> {
        Box::pin(async { Err(2) })
    }
    fn ping<'a>(&'a self) -> Pin<Box<dyn Future<Output = u32> + Send + 'a>> {
        Box::pin(async { 2 })
    }
}
pub enum Any {
    Memory(Memory),
    Disk(Disk),
}
#[automatically_derived]
impl ::core::convert::From<Memory> for Any {
    #[inline]
    fn from(__value: Memory) -> Self {
        Self::Memory(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Disk> for Any {
    #[inline]
    fn from(__value: Disk) -> Self {
        Self::Disk(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Any_c2689f579e77560a_53_1 as Any;
#[automatically_derived]
impl Store for Any {
    type Error = Unified;
    #[inline]
    fn load<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<u32, Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            match self {
                Any::Memory(__inner) => {
                    <Memory as Store>::load(__inner)
                        .await
                        .map_err(::core::convert::Into::into)
                }
                Any::Disk(__inner) => {
                    <Disk as Store>::load(__inner)
                        .await
                        .map_err(::core::convert::Into::into)
                }
            }
        })
    }
    #[inline]
    fn ping<'a>(&'a self) -> Pin<Box<dyn Future<Output = u32> + Send + 'a>> {
        match self {
            Any::Memory(__inner) => <Memory as Store>::ping(__inner),
            Any::Disk(__inner) => <Disk as Store>::ping(__inner),
        }
    }
}
fn main() {}
