//! The support module every shard holds: the types and the executor the
//! cells share.

/// The source of a shard's `support` module.
pub const SOURCE: &str = r#"//! What every cell of the shard shares.

use core::future::Future;
use core::task::{Context, Poll, Waker};

/// What an inherent method of a payload answers: a forwarding that calls it
/// instead of the trait's method shows in the law.
pub const POISON: u64 = 999_999_999;

/// The place a poisoned reference points at.
pub static POISON_PLACE: u64 = POISON;

/// The place a mismatched reference points at.
pub static MISMATCH: u64 = 424_242;

/// The error a payload answers with.
#[derive(Debug, PartialEq, Eq)]
pub struct Fault(pub u64);

/// What every payload's own associated type converts into, the type the
/// enum overrides an associated type with.
#[derive(Debug, PartialEq, Eq)]
pub struct Wide {
    /// The slot of the payload whose value it was.
    pub slot: u64,
    /// The value.
    pub value: u64,
}

impl Wide {
    /// The mismatch of a method returning the overridden type.
    pub const MISMATCH: Wide = Wide {
        slot: 9,
        value: 424_242,
    };
}

/// The associated type of `P0`.
#[derive(Debug, PartialEq, Eq)]
pub struct Narrow0(pub u64);

/// The associated type of `P1`.
#[derive(Debug, PartialEq, Eq)]
pub struct Narrow1(pub u64);

impl From<Narrow0> for Wide {
    fn from(narrow: Narrow0) -> Wide {
        Wide {
            slot: 0,
            value: narrow.0,
        }
    }
}

impl From<Narrow1> for Wide {
    fn from(narrow: Narrow1) -> Wide {
        Wide {
            slot: 1,
            value: narrow.0,
        }
    }
}

/// A type parameter no argument fixes.
pub trait Weight {
    /// What the answer counts it as.
    const VALUE: u64;
}

/// The weight the law calls with.
pub struct Three;

impl Weight for Three {
    const VALUE: u64 = 3;
}

/// What a function lift counts a value as.
pub trait Counted {
    /// Returns the count.
    fn count(&self) -> u64;
}

impl Counted for u64 {
    fn count(&self) -> u64 {
        *self
    }
}

/// A lift taking its value by reference.
pub fn by_ref<T: Counted>(value: &T) -> u64 {
    value.count() * 2
}

/// A lift taking its value by value.
pub fn by_value<T: Counted>(value: T) -> u64 {
    value.count() * 3
}

/// Polls a future to completion; the cells' futures never wait. The
/// future is boxed rather than pinned with `pin!`, whose expansion holds
/// syntax `syn` does not read, so that `cargo expand` output parses.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
}
"#;
