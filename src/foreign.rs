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
//! that are stable on the crate's minimum Rust version. Adapters that
//! consume `Self` into a wrapper, such as `Iterator::map` or `Read::bytes`,
//! keep their defaults, which call the forwarded methods. The `std`
//! declarations are plain tokens here and resolve in the dispatching crate,
//! so this module needs no feature; a `no_std` crate uses the `core` ones.
//!
//! | Declaration | Implements | Forwarded |
//! | --- | --- | --- |
//! | `Display` | `core::fmt::Display` | `fmt` |
//! | `Debug` | `core::fmt::Debug` | `fmt` |
//! | `Error` | `core::error::Error` | `source` |
//! | `PartialEq` | `core::cmp::PartialEq` | `eq` and `ne`; two different variants are never equal |
//! | `Iterator` | `core::iter::Iterator` | `type Item`, `next`, `size_hint`, `count`, `last`, `nth`, `fold` |
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
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: ::core::ops::FnMut(B, Self::Item) -> B;
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
