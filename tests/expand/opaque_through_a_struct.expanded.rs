//! A struct forwards a return-position `impl Trait` as it is: the `match`
//! has one arm, so there is one opaque type. A future whose output needs
//! nothing is forwarded the same way; one whose output is rewrapped is
//! awaited in an `async` block, where the rewrap happens.
use std::future::Future;
pub trait Source {
    fn items(&self) -> impl Iterator<Item = u32> + '_;
    fn load(&self) -> impl Future<Output = u32> + Send;
    fn reloaded(&self) -> impl Future<Output = Self> + Send
    where
        Self: Sized;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Source_f145ff8a078b8a31_7_1 as Source;
pub struct Few(pub Vec<u32>);
impl Source for Few {
    fn items(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied()
    }
    fn load(&self) -> impl Future<Output = u32> + Send {
        async move { self.0.len() as u32 }
    }
    fn reloaded(&self) -> impl Future<Output = Self> + Send {
        async move { Few(self.0.clone()) }
    }
}
pub struct Wrapped(Few);
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Wrapped_55ba79500a51008b_30_1 as Wrapped;
#[automatically_derived]
impl Source for Wrapped {
    #[inline]
    fn items(&self) -> impl Iterator<Item = u32> + '_ {
        match self {
            Wrapped(__inner, ..) => <Few as Source>::items(__inner),
        }
    }
    #[inline]
    fn load(&self) -> impl Future<Output = u32> + Send {
        match self {
            Wrapped(__inner, ..) => <Few as Source>::load(__inner),
        }
    }
    #[inline]
    fn reloaded(&self) -> impl Future<Output = Self> + Send
    where
        Self: Sized,
    {
        async move {
            match self {
                Wrapped(__inner, ..) => Wrapped(<Few as Source>::reloaded(__inner).await),
            }
        }
    }
}
fn main() {}
