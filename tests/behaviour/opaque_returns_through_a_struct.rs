//! A struct forwards a return-position `impl Trait` as it is: its `match`
//! has one arm, so there is one opaque type.

use std::future::Future;

#[whichever::dispatchable]
pub trait Source {
    fn items(&self) -> impl Iterator<Item = u32> + '_;
    fn load(&self) -> impl Future<Output = u32> + Send;
    fn reloaded(&self) -> impl Future<Output = Self> + Send
    where
        Self: Sized;
}

#[derive(Debug, PartialEq)]
pub struct Few(pub Vec<u32>);

// The trait spells its futures as `impl Future`, as a public trait that
// has to promise `Send` does; the impl follows it.
#[allow(clippy::manual_async_fn)]
impl Source for Few {
    fn items(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied()
    }
    fn load(&self) -> impl Future<Output = u32> + Send {
        async move { self.0.len() as u32 }
    }
    fn reloaded(&self) -> impl Future<Output = Self> + Send {
        async move { Few(self.0.iter().rev().copied().collect()) }
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(Source)]
pub struct Wrapped(Few);

// An inherent method of a struct forwards an opaque type the same way.
#[derive(Debug, PartialEq)]
#[whichever::dispatch]
pub struct Labelled {
    #[dispatch]
    few: Few,
    label: &'static str,
}

#[whichever::dispatch]
impl Labelled {
    #[dispatch(*, call = items)]
    pub fn items_again(&self) -> impl Iterator<Item = u32> + '_;
}

#[test]
fn the_one_arm_returns_the_opaque_type() {
    let wrapped = Wrapped(Few(vec![1, 2, 3]));
    assert_eq!(wrapped.items().sum::<u32>(), 6);
    assert_eq!(pollster::block_on(wrapped.load()), 3);
    assert_eq!(
        pollster::block_on(wrapped.reloaded()),
        Wrapped(Few(vec![3, 2, 1]))
    );
    let labelled = Labelled {
        few: Few(vec![4]),
        label: "a",
    };
    assert_eq!(labelled.items_again().sum::<u32>(), 4);
    assert_eq!(labelled.label, "a");
}
