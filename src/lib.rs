//! Implements a trait for an enum by forwarding to whichever variant it holds.
//!
//! `#[dispatchable]` goes on the trait, in the crate that owns it.
//! `#[dispatch(Trait)]` goes on an enum whose variants each hold one value
//! implementing the trait, in any crate. The enum then implements the trait
//! with one `match` per method, each arm a fully qualified call into the
//! variant's payload, and gets a `From<Payload>` impl per variant.
//!
//! ```
//! // In the crate that owns the trait.
//! #[whichever::dispatchable]
//! pub trait Shape {
//!     fn area(&self) -> u32;
//!     fn name(&self) -> &'static str {
//!         "shape"
//!     }
//! }
//!
//! pub struct Square(pub u32);
//! pub struct Rect(pub u32, pub u32);
//!
//! impl Shape for Square {
//!     fn area(&self) -> u32 {
//!         self.0 * self.0
//!     }
//!     fn name(&self) -> &'static str {
//!         "square"
//!     }
//! }
//!
//! impl Shape for Rect {
//!     fn area(&self) -> u32 {
//!         self.0 * self.1
//!     }
//! }
//!
//! // In any crate that owns an enum over implementors.
//! #[whichever::dispatch(Shape)]
//! pub enum AnyShape {
//!     Square(Square),
//!     Rect(Rect),
//! }
//!
//! fn main() {
//!     let square: AnyShape = Square(3).into();
//!     let rect: AnyShape = Rect(2, 5).into();
//!     assert_eq!((square.area(), square.name()), (9, "square"));
//!     assert_eq!((rect.area(), rect.name()), (10, "shape"));
//! }
//! ```
//!
//! # What is generated
//!
//! For the enum above, `#[dispatch(Shape)]` expands to the impl a careful
//! hand would write, plus `#[inline]` and the conversions:
//!
//! ```rust,ignore
//! impl Shape for AnyShape {
//!     #[inline]
//!     fn area(&self) -> u32 {
//!         match self {
//!             AnyShape::Square(inner) => <Square as Shape>::area(inner),
//!             AnyShape::Rect(inner) => <Rect as Shape>::area(inner),
//!         }
//!     }
//!     #[inline]
//!     fn name(&self) -> &'static str {
//!         match self {
//!             AnyShape::Square(inner) => <Square as Shape>::name(inner),
//!             AnyShape::Rect(inner) => <Rect as Shape>::name(inner),
//!         }
//!     }
//! }
//!
//! impl From<Square> for AnyShape { /* .. */ }
//! impl From<Rect> for AnyShape { /* .. */ }
//! ```
//!
//! Default methods are always forwarded, so a variant's override is never
//! hidden by the trait's default at the enum level. The calls are fully
//! qualified, so an inherent method on a payload with the same name as a
//! trait method does not take over. There is no trait object, no closure
//! and no conversion in the arms unless an option asks for one.
//!
//! # Options
//!
//! All options follow the trait path in `#[dispatch(..)]`:
//!
//! - `Name = Type` fixes the associated type `Name` instead of taking it
//!   from the first variant. A method returning `Self::Name` is then
//!   converted with [`Into`], and one returning `Result<_, Self::Name>` with
//!   `map_err(Into::into)`, so variants whose own type converts into the
//!   named one can share the enum. See [associated types](#associated-types).
//! - `mismatch(method = expr, ..)` gives the result of `method` when its
//!   `Self` arguments hold a different variant than `self`. The trait author
//!   can decide this once instead, with `#[dispatch(mismatch = expr)]` on
//!   the method. See [`Self` arguments](#self-arguments).
//! - `from = false` suppresses the `From<Payload>` impls.
//!
//! Several traits are dispatched by repeating the attribute, one trait per
//! attribute; a supertrait is just one more attribute:
//!
//! ```rust,ignore
//! #[whichever::dispatch(SmartContract)]
//! #[whichever::dispatch(SwapSmartContract)]
//! pub enum ContractKind { /* .. */ }
//! ```
//!
//! # Semantics
//!
//! | Trait item | Enum impl |
//! | --- | --- |
//! | `type X;` | `type X = <FirstVariant as Trait>::X;`, or the `X = Type` option. Every other variant is checked to agree; a disagreement is reported on that variant. |
//! | `const N: T;` | `const N: T = <FirstVariant as Trait>::N;` |
//! | method with a receiver | A `match` over the variants calling `<Payload as Trait>::method(payload, args..)`. |
//! | default body | Ignored: the method is forwarded like any other. |
//! | `async fn` | Each arm is awaited. |
//! | `unsafe fn` | Each call is wrapped in `unsafe { }`. |
//! | argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`; different variants give the `mismatch` expression. |
//! | return `Self` | Wrapped back into the same variant; `Option<Self>` and `Result<Self, E>` are mapped. |
//! | return `Self::X` with `X` overridden | Converted with `Into`; `Result<_, Self::X>` with `map_err(Into::into)`. |
//! | method without a receiver | Left to the trait's default body; an error when there is none. |
//! | `#[cfg]` on a method or variant | Carried onto the generated method or arm. |
//! | `#[inline]` | Added to every generated method unless the trait method carries its own. |
//!
//! # Generics
//!
//! A generic trait is dispatched at a concrete instantiation, named on the
//! enum: `#[dispatch(Settler<Solaris>)]`. Type, lifetime and const
//! parameters are substituted in every signature, and a projection such as
//! `W::Plan` becomes `<Solaris as World>::Plan` when `W` has exactly one
//! bound. The enum itself may be generic; its parameters, bounds and `where`
//! clause carry into the impl.
//!
//! # `Self` arguments
//!
//! A method such as `fn pricing_eq(&self, other: &Self) -> bool` compares
//! two values of the enum. The generated method matches both on the same
//! variant and calls the payload's method; for two different variants it
//! returns the expression given by `#[dispatch(mismatch = false)]` on the
//! trait method, or by `mismatch(pricing_eq = false)` on the enum. Without
//! one the expansion fails with an error naming the method. Only `Self`,
//! `&Self` and `&mut Self` are accepted; a `Self` nested in another type
//! (`Option<&Self>`) is an error.
//!
//! # Associated types
//!
//! Without an override the enum takes each associated type from its first
//! variant and a hidden check asserts that every other variant uses the
//! same type, with the error on the variant that differs. With
//! `Error = NetworkError` the enum's type is `NetworkError`, and each
//! variant's own error type only has to implement `Into<NetworkError>`:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Connector {
//!     type Error;
//!     fn connect(&self) -> Result<u32, Self::Error>;
//! }
//!
//! #[derive(Debug, PartialEq)]
//! pub struct NetworkError(pub String);
//!
//! pub struct Mainnet;
//! pub struct Test;
//!
//! impl Connector for Mainnet {
//!     type Error = NetworkError;
//!     fn connect(&self) -> Result<u32, NetworkError> {
//!         Ok(1)
//!     }
//! }
//!
//! impl Connector for Test {
//!     type Error = &'static str;
//!     fn connect(&self) -> Result<u32, &'static str> {
//!         Err("offline")
//!     }
//! }
//!
//! impl From<&'static str> for NetworkError {
//!     fn from(message: &'static str) -> Self {
//!         NetworkError(message.to_owned())
//!     }
//! }
//!
//! #[whichever::dispatch(Connector, Error = NetworkError)]
//! pub enum AnyConnector {
//!     Mainnet(Mainnet),
//!     Test(Test),
//! }
//!
//! fn main() {
//!     let test: AnyConnector = Test.into();
//!     assert_eq!(test.connect(), Err(NetworkError("offline".to_owned())));
//! }
//! ```
//!
//! # Across crates
//!
//! `#[dispatchable]` emits the trait unchanged plus a hidden exported macro
//! carrying the trait's tokens, re-exported under the trait's own name
//! (macros live in their own namespace, so `use engine::Swap;` imports both).
//! `#[dispatch(Swap)]` invokes that macro, which hands the trait and the
//! enum to a hidden proc macro that writes the impl. Nothing is cached
//! between expansions, so the two attributes work across crates, in any
//! order, and under rust-analyzer. Both crates depend on `whichever`; a
//! renamed dependency is fine.
//!
//! The types named in the trait's signatures are written into the enum's
//! module, so they must be in scope there, exactly as for a hand-written
//! impl. Paths starting with `crate::` are rewritten to the trait's crate;
//! `super::` and `self::` paths cannot be and must not appear in a
//! dispatchable trait's signatures.
//!
//! # Requirements and limits
//!
//! - Every variant holds exactly one unnamed field, `Name(Payload)`, and
//!   every payload implements the trait. A payload type used by two variants
//!   gets no `From` impl.
//! - A method without a receiver needs a default body; the enum uses it.
//! - Return-position `impl Trait` cannot be dispatched by a `match`: every
//!   variant would return a different opaque type.
//! - `Box<T>`, `Arc<T>` and `&T` payloads are not dereferenced for the call
//!   (planned as a variant option).
//! - Generic associated types and `self: Type` receivers are not supported.
//! - Associated consts are taken from the first variant without a check.
//!
//! "cannot find macro `Swap` in this scope" at the enum means the trait is
//! not marked `#[dispatchable]`, or the trait (which carries the macro) is
//! not imported where the enum is.
//!
//! The crate is `no_std`: the generated code names only `core` items. The
//! minimum supported Rust version is 1.88.
#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub use whichever_macros::{dispatch, dispatchable};

#[doc(hidden)]
pub mod __private {
    pub use whichever_macros::implement;
}
