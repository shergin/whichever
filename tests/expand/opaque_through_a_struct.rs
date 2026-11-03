//! A struct forwards a return-position `impl Trait` as it is: the `match`
//! has one arm, so there is one opaque type. A future whose output needs
//! nothing is forwarded the same way; one whose output is rewrapped is
//! awaited in an `async` block, where the rewrap happens.
use std::future::Future;

#[whichever::dispatchable]
pub trait Source {
    fn items(&self) -> impl Iterator<Item = u32> + '_;
    fn load(&self) -> impl Future<Output = u32> + Send;
    fn reloaded(&self) -> impl Future<Output = Self> + Send
    where
        Self: Sized;
}

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

#[whichever::dispatch(Source)]
pub struct Wrapped(Few);

fn main() {}
