//! Declarations of standard library traits for `#[dispatch]`.
//!
//! A trait from `core` or `std` is not `#[dispatchable]`, so this module
//! restates the ones an enum over implementors most often forwards by hand.
//! Each is a `#[dispatchable(remote = ..)]` declaration: it emits no trait,
//! only the carrier macro that `#[dispatch]` reads, and the impl it generates
//! is of the real trait. `#[dispatch(whichever::foreign::Display)]` on an
//! enum implements `core::fmt::Display` for it, with one `match` whose arms
//! are `<Payload as core::fmt::Display>::fmt(inner, f)`.
//!
//! ```
//! use std::io;
//!
//! #[derive(Debug)]
//! #[whichever::dispatch(whichever::foreign::Display)]
//! #[whichever::dispatch(whichever::foreign::Error)]
//! pub enum ReadError {
//!     Io(io::Error),
//!     Parse(std::num::ParseIntError),
//! }
//!
//! fn main() {
//!     let error: ReadError = "x".parse::<u32>().unwrap_err().into();
//!     assert_eq!(error.to_string(), "invalid digit found in string");
//! }
//! ```
//!
//! The forwarded provided methods are the ones a `match` can forward and
//! that are stable on the crate's minimum Rust version, so a payload's
//! override is what runs through the enum: `core`'s slice iterators answer
//! `any`, `all`, `find` and `position` themselves, a range answers `max`,
//! `min` and `is_sorted`, and a `vec::IntoIter` collected into a `Vec`
//! reuses its allocation. Adapters that consume `Self` into a wrapper, such
//! as `Iterator::map` or `Read::bytes`, and `by_ref` keep their defaults,
//! which call the forwarded methods. `Iterator` also leaves out `unzip`,
//! which a `match` cannot forward, since the bound `Self: Iterator<Item =
//! (A, B)>` does not reach the payload; `rposition`, whose bound
//! `Self: ExactSizeIterator + DoubleEndedIterator` holds only for an enum
//! that is both; and the methods that are unstable on that version, such as
//! `try_fold`, `try_for_each` and `advance_by`. `Error` leaves out
//! `description` and `cause`, which are deprecated: a hand does not forward
//! them, and a partial block over `Error` that answers `source` itself
//! would then have to answer them too.
//!
//! Every method the `Iterator` declaration forwards but `next`, `size_hint`
//! and `nth` requires `Self: Sized`, as `core` declares them, and so do
//! `DoubleEndedIterator`'s `rfold` and `rfind`. A `Box<dyn Iterator>` is itself an `Iterator`, so an
//! enum holds one as a plain payload, without `deref`, and every method is
//! forwarded to the box; the same holds for a `Box<dyn DoubleEndedIterator>`
//! under `DoubleEndedIterator` and a `Box<dyn ExactSizeIterator>` under
//! `ExactSizeIterator`. `DoubleEndedIterator` and `ExactSizeIterator` are
//! dispatched beside `Iterator`, their supertrait, one attribute each.
//!
//! An override such as `Item = Wide` converts what a method returns, not an
//! argument that names the type, so the methods that take a closure over
//! `Self::Item` compile only where every payload's `Item` is `Wide`. Where
//! the payloads' items differ, restate `Iterator` in the dispatching crate
//! with the methods to forward, `next` and `size_hint`, and the rest take
//! the trait's default:
//!
//! ```
//! use std::ops::Range;
//!
//! #[whichever::dispatchable(remote = core::iter::Iterator)]
//! trait Iterator {
//!     type Item;
//!     fn next(&mut self) -> Option<Self::Item>;
//!     fn size_hint(&self) -> (usize, Option<usize>);
//! }
//!
//! #[whichever::dispatch(Iterator, Item = u64)]
//! pub enum Either {
//!     Narrow(Range<u32>),
//!     Broad(Range<u64>),
//! }
//!
//! fn main() {
//!     let both = Either::Narrow(1..3).chain(Either::Broad(7..8));
//!     assert_eq!(both.collect::<Vec<u64>>(), [1, 2, 7]);
//! }
//! ```
//!
//! The `std` declarations are plain tokens here and resolve in the
//! dispatching crate, so this module needs no feature; a `no_std` crate
//! uses the `core` ones.
//!
//! | Declaration | Implements | Forwarded |
//! | --- | --- | --- |
//! | `Display` | `core::fmt::Display` | `fmt` |
//! | `Debug` | `core::fmt::Debug` | `fmt` |
//! | `Error` | `core::error::Error` | `source` |
//! | `PartialEq` | `core::cmp::PartialEq` | `eq` and `ne`; two different variants are never equal |
//! | `Iterator` | `core::iter::Iterator` | `type Item`, `next`, `size_hint`, `count`, `last`, `nth`, `for_each`, `collect`, `partition`, `fold`, `reduce`, `all`, `any`, `find`, `find_map`, `position`, `max`, `min`, `max_by_key`, `max_by`, `min_by_key`, `min_by`, `sum`, `product`, `cmp`, `partial_cmp`, `eq`, `ne`, `lt`, `le`, `gt`, `ge`, `is_sorted`, `is_sorted_by`, `is_sorted_by_key` |
//! | `DoubleEndedIterator` | `core::iter::DoubleEndedIterator` | `next_back`, `nth_back`, `rfold`, `rfind` |
//! | `ExactSizeIterator` | `core::iter::ExactSizeIterator` | `len` |
//! | `Read` | `std::io::Read` | `read`, `read_vectored`, `read_to_end`, `read_to_string`, `read_exact` |
//! | `Write` | `std::io::Write` | `write`, `write_vectored`, `flush`, `write_all`, `write_fmt` |
//! | `BufRead` | `std::io::BufRead` | `fill_buf`, `consume`, `read_until`, `skip_until`, `read_line` |
//! | `Seek` | `std::io::Seek` | `seek`, `rewind`, `stream_position`, `seek_relative` |
//!
//! Any other trait is restated the same way in the dispatching crate; see
//! the crate documentation.
#![allow(clippy::module_name_repetitions)]

use whichever_macros::dispatchable;

#[dispatchable(remote = ::core::fmt::Display)]
pub trait Display {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result;
}

#[dispatchable(remote = ::core::fmt::Debug)]
pub trait Debug {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result;
}

#[dispatchable(remote = ::core::error::Error)]
pub trait Error {
    fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)>;
}

#[dispatchable(remote = ::core::cmp::PartialEq)]
pub trait PartialEq {
    #[dispatch(mismatch = false)]
    fn eq(&self, other: &Self) -> bool;
    #[dispatch(mismatch = true)]
    fn ne(&self, other: &Self) -> bool;
}

#[dispatchable(remote = ::core::iter::Iterator)]
pub trait Iterator {
    type Item;
    fn next(&mut self) -> ::core::option::Option<Self::Item>;
    fn size_hint(&self) -> (usize, ::core::option::Option<usize>);
    fn count(self) -> usize
    where
        Self: Sized;
    fn last(self) -> ::core::option::Option<Self::Item>
    where
        Self: Sized;
    fn nth(&mut self, n: usize) -> ::core::option::Option<Self::Item>;
    fn for_each<F>(self, f: F)
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item);
    fn collect<B: ::core::iter::FromIterator<Self::Item>>(self) -> B
    where
        Self: Sized;
    fn partition<B, F>(self, f: F) -> (B, B)
    where
        Self: Sized,
        B: ::core::default::Default + ::core::iter::Extend<Self::Item>,
        F: ::core::ops::FnMut(&Self::Item) -> bool;
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: ::core::ops::FnMut(B, Self::Item) -> B;
    fn reduce<F>(self, f: F) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item, Self::Item) -> Self::Item;
    fn all<F>(&mut self, f: F) -> bool
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item) -> bool;
    fn any<F>(&mut self, f: F) -> bool
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item) -> bool;
    fn find<P>(&mut self, predicate: P) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        P: ::core::ops::FnMut(&Self::Item) -> bool;
    fn find_map<B, F>(&mut self, f: F) -> ::core::option::Option<B>
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item) -> ::core::option::Option<B>;
    fn position<P>(&mut self, predicate: P) -> ::core::option::Option<usize>
    where
        Self: Sized,
        P: ::core::ops::FnMut(Self::Item) -> bool;
    fn max(self) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        Self::Item: ::core::cmp::Ord;
    fn min(self) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        Self::Item: ::core::cmp::Ord;
    fn max_by_key<B: ::core::cmp::Ord, F>(self, f: F) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item) -> B;
    fn max_by<F>(self, compare: F) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item, &Self::Item) -> ::core::cmp::Ordering;
    fn min_by_key<B: ::core::cmp::Ord, F>(self, f: F) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item) -> B;
    fn min_by<F>(self, compare: F) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item, &Self::Item) -> ::core::cmp::Ordering;
    fn sum<S>(self) -> S
    where
        Self: Sized,
        S: ::core::iter::Sum<Self::Item>;
    fn product<P>(self) -> P
    where
        Self: Sized,
        P: ::core::iter::Product<Self::Item>;
    fn cmp<I>(self, other: I) -> ::core::cmp::Ordering
    where
        I: ::core::iter::IntoIterator<Item = Self::Item>,
        Self::Item: ::core::cmp::Ord,
        Self: Sized;
    fn partial_cmp<I>(self, other: I) -> ::core::option::Option<::core::cmp::Ordering>
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized;
    fn eq<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialEq<I::Item>,
        Self: Sized;
    fn ne<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialEq<I::Item>,
        Self: Sized;
    fn lt<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized;
    fn le<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized;
    fn gt<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized;
    fn ge<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized;
    fn is_sorted(self) -> bool
    where
        Self: Sized,
        Self::Item: ::core::cmp::PartialOrd;
    fn is_sorted_by<F>(self, compare: F) -> bool
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item, &Self::Item) -> bool;
    fn is_sorted_by_key<F, K>(self, f: F) -> bool
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item) -> K,
        K: ::core::cmp::PartialOrd;
}

#[dispatchable(remote = ::core::iter::DoubleEndedIterator)]
pub trait DoubleEndedIterator: ::core::iter::Iterator {
    fn next_back(&mut self) -> ::core::option::Option<Self::Item>;
    fn nth_back(&mut self, n: usize) -> ::core::option::Option<Self::Item>;
    fn rfold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: ::core::ops::FnMut(B, Self::Item) -> B;
    fn rfind<P>(&mut self, predicate: P) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        P: ::core::ops::FnMut(&Self::Item) -> bool;
}

#[dispatchable(remote = ::core::iter::ExactSizeIterator)]
pub trait ExactSizeIterator: ::core::iter::Iterator {
    fn len(&self) -> usize;
}

#[dispatchable(remote = ::std::io::Read)]
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize>;
    fn read_vectored(&mut self, bufs: &mut [::std::io::IoSliceMut<'_>])
        -> ::std::io::Result<usize>;
    fn read_to_end(&mut self, buf: &mut ::std::vec::Vec<u8>) -> ::std::io::Result<usize>;
    fn read_to_string(&mut self, buf: &mut ::std::string::String) -> ::std::io::Result<usize>;
    fn read_exact(&mut self, buf: &mut [u8]) -> ::std::io::Result<()>;
}

#[dispatchable(remote = ::std::io::Write)]
pub trait Write {
    fn write(&mut self, buf: &[u8]) -> ::std::io::Result<usize>;
    fn write_vectored(&mut self, bufs: &[::std::io::IoSlice<'_>]) -> ::std::io::Result<usize>;
    fn flush(&mut self) -> ::std::io::Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> ::std::io::Result<()>;
    fn write_fmt(&mut self, fmt: ::core::fmt::Arguments<'_>) -> ::std::io::Result<()>;
}

#[dispatchable(remote = ::std::io::BufRead)]
pub trait BufRead {
    fn fill_buf(&mut self) -> ::std::io::Result<&[u8]>;
    fn consume(&mut self, amt: usize);
    fn read_until(&mut self, byte: u8, buf: &mut ::std::vec::Vec<u8>) -> ::std::io::Result<usize>;
    fn skip_until(&mut self, byte: u8) -> ::std::io::Result<usize>;
    fn read_line(&mut self, buf: &mut ::std::string::String) -> ::std::io::Result<usize>;
}

#[dispatchable(remote = ::std::io::Seek)]
pub trait Seek {
    fn seek(&mut self, pos: ::std::io::SeekFrom) -> ::std::io::Result<u64>;
    fn rewind(&mut self) -> ::std::io::Result<()>;
    fn stream_position(&mut self) -> ::std::io::Result<u64>;
    fn seek_relative(&mut self, offset: i64) -> ::std::io::Result<()>;
}
