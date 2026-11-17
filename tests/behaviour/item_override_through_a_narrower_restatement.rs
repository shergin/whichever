//! An override converts what a method returns, so `next` hands back the
//! override's type, but an argument that names the overridden type is
//! handed to the payload as it is: `fold`'s closure over `Self::Item` would
//! be a closure over `Wide` where the payload's takes `u32`. A restatement
//! forwards exactly what it lists, so one of `next` and `size_hint` alone
//! leaves every method that takes such a closure to the real trait's
//! default, which calls `next`.
use std::ops::Range;

#[derive(Debug, PartialEq)]
pub struct Wide(pub u64);

impl From<u32> for Wide {
    fn from(value: u32) -> Self {
        Wide(u64::from(value))
    }
}

impl From<u64> for Wide {
    fn from(value: u64) -> Self {
        Wide(value)
    }
}

#[whichever::dispatchable(remote = core::iter::Iterator)]
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
    fn size_hint(&self) -> (usize, Option<usize>);
}

#[whichever::dispatch(Iterator, Item = Wide)]
pub enum Either {
    Narrow(Range<u32>),
    Broad(Range<u64>),
}

#[test]
fn the_items_are_converted_and_the_rest_takes_the_default() {
    let mut narrow = Either::Narrow(1..3);
    assert_eq!(narrow.size_hint(), (2, Some(2)));
    assert_eq!(narrow.next(), Some(Wide(1)));
    let broad = Either::Broad(u64::MAX - 1..u64::MAX);
    let all: Vec<Wide> = narrow.chain(broad).collect();
    assert_eq!(all, [Wide(2), Wide(u64::MAX - 1)]);
    let total = Either::Narrow(1..4).fold(0, |sum, Wide(number)| sum + number);
    assert_eq!(total, 6);
    assert!(Either::Broad(5..7).any(|Wide(number)| number == 6));
}

/// An override that names the payloads' own type converts nothing, so
/// `foreign::Iterator` forwards every method with it.
#[whichever::dispatch(whichever::foreign::Iterator, Item = u8)]
pub enum Bytes {
    Owned(std::vec::IntoIter<u8>),
    Single(std::option::IntoIter<u8>),
}

#[test]
fn an_override_of_the_payloads_own_type_forwards_every_method() {
    assert_eq!(
        Bytes::Owned(vec![1, 2, 3].into_iter()).fold(0, |total, byte| total * 10 + byte),
        123
    );
    assert_eq!(Bytes::Single(Some(4).into_iter()).max(), Some(4));
}
