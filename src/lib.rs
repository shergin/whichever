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
//! - `Name = value` fixes the associated const `Name` instead of taking it
//!   from the first variant, for a const that differs per payload by
//!   design, such as a kind discriminator.
//! - `mismatch(method = expr, ..)` gives the result of `method` when its
//!   `Self` arguments hold a different variant than `self`. The trait author
//!   can decide this once instead, with `#[dispatch(mismatch = expr)]` on
//!   the method. See [`Self` arguments](#self-arguments).
//! - `from = false` suppresses the `From<Payload>` impls.
//! - `try_from = true` adds `TryFrom<Enum> for Payload` for every payload
//!   type, with the enum itself as the error, so a mismatch hands the value
//!   back.
//!
//! Three options go on a variant. `#[dispatch(from = false)]` leaves the
//! variant out of the `From` and `TryFrom` impls, for a payload such as
//! `String` that should not convert into the enum, and
//! `#[dispatch(try_from = true)]` adds the `TryFrom` impl for that payload
//! alone. `#[dispatch(deref)]` calls the trait on what the payload
//! dereferences to, so `Box<T>`, `Rc<T>`, `Arc<T>`, `&T` and `&mut T`
//! payloads dispatch like a plain `T`. See
//! [dereferenced payloads](#dereferenced-payloads).
//!
//! Several traits are dispatched by repeating the attribute, one trait per
//! attribute; a supertrait is just one more attribute:
//!
//! ```rust,ignore
//! #[whichever::dispatch(Shape)]
//! #[whichever::dispatch(Drawable)]
//! pub enum AnyShape { /* .. */ }
//! ```
//!
//! # Semantics
//!
//! | Trait item | Enum impl |
//! | --- | --- |
//! | `type X;` | `type X = <FirstVariant as Trait>::X;`, or the `X = Type` option. Every other variant is checked to agree; a disagreement is reported on that variant. |
//! | `const N: T;` | `const N: T = <FirstVariant as Trait>::N;`, or the `N = value` option. For an integer, `bool`, `char` or `&str` const on a non-generic enum, every other variant is checked to agree at compile time, with the error on that variant. |
//! | method with a receiver | A `match` over the variants calling `<Payload as Trait>::method(payload, args..)`. |
//! | `self: Box<Self>` | The enum is moved out of its box and a `deref` variant's own `Box<Payload>` is handed over; a plain variant is an error. |
//! | `self: Pin<&Self>`, `self: Pin<&mut Self>` | The enum is unpinned, matched, and the payload pinned again; every payload must be `Unpin`. |
//! | default body | Ignored: the method is forwarded like any other. |
//! | `async fn` | Each arm is awaited. |
//! | `-> impl Future<Output = T>` | The `match` sits in one `async move` block and each arm is awaited; the future's bounds, `Send` included, are checked against that block. |
//! | `unsafe fn` | Each call is wrapped in `unsafe { }`. |
//! | argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`; different variants give the `mismatch` expression. |
//! | return `Self` | Wrapped back into the same variant; `Option<Self>` and `Result<Self, E>` are mapped, under any path, `io::Result<Self>` included. `Self` anywhere else in a return type is an error. |
//! | return `Self::X` with `X` overridden | Converted with `Into`; `Result<_, Self::X>` with `map_err(Into::into)`. |
//! | method without a receiver | Left to the trait's default body, or written in a `#[dispatch] impl Trait for Enum` block; an error when it has neither. |
//! | `#[cfg]` on a method or variant | Carried onto the generated method or arm. |
//! | `#[inline]` | Added to every generated method unless the trait method carries its own. |
//!
//! # Generics
//!
//! A generic trait is dispatched at a concrete instantiation, named on the
//! enum: `#[dispatch(Encoder<Json>)]`. Type, lifetime and const parameters
//! are substituted in every signature, and a projection such as `F::Value`
//! becomes `<Json as Format>::Value` when `F` has exactly one bound. The
//! enum itself may be generic; its parameters, bounds and `where` clause
//! carry into the impl.
//!
//! # `Self` arguments
//!
//! A method such as `fn congruent(&self, other: &Self) -> bool` compares two
//! values of the enum. The generated method matches both on the same variant
//! and calls the payload's method; for two different variants it returns the
//! expression given by `#[dispatch(mismatch = false)]` on the trait method,
//! or by `mismatch(congruent = false)` on the enum. Without one the expansion
//! fails with an error naming the method. Only `Self`, `&Self` and
//! `&mut Self` are accepted; a `Self` nested in another type
//! (`Option<&Self>`) is an error.
//!
//! # Associated types
//!
//! Without an override the enum takes each associated type from its first
//! variant, and a hidden check asserts that every other variant uses the
//! same type, with the error on the variant that differs. With
//! `Error = StorageError` the enum's type is `StorageError`, and each
//! variant's own error type only has to implement `Into<StorageError>`:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Storage {
//!     type Error;
//!     fn load(&self, key: &str) -> Result<u32, Self::Error>;
//! }
//!
//! #[derive(Debug, PartialEq)]
//! pub struct StorageError(pub String);
//!
//! pub struct Memory;
//! pub struct Disk;
//!
//! impl Storage for Memory {
//!     type Error = StorageError;
//!     fn load(&self, key: &str) -> Result<u32, StorageError> {
//!         Ok(key.len() as u32)
//!     }
//! }
//!
//! impl Storage for Disk {
//!     type Error = &'static str;
//!     fn load(&self, key: &str) -> Result<u32, &'static str> {
//!         Err("offline")
//!     }
//! }
//!
//! impl From<&'static str> for StorageError {
//!     fn from(message: &'static str) -> Self {
//!         StorageError(message.to_owned())
//!     }
//! }
//!
//! #[whichever::dispatch(Storage, Error = StorageError)]
//! pub enum AnyStorage {
//!     Memory(Memory),
//!     Disk(Disk),
//! }
//!
//! fn main() {
//!     let disk: AnyStorage = Disk.into();
//!     assert_eq!(disk.load("answer"), Err(StorageError("offline".to_owned())));
//! }
//! ```
//!
//! # Dereferenced payloads
//!
//! A variant marked `#[dispatch(deref)]` holds its value behind a pointer or
//! a reference. The trait is called on `<Payload as Deref>::Target`, with
//! the receiver and any `Self` argument dereferenced to match: `&**inner`
//! for `&self`, `&mut **inner` for `&mut self` (the payload must implement
//! `DerefMut`), `*inner` for `self` (a `Box` moves out, a reference cannot).
//! A method returning `Self` cannot wrap its result back into such a
//! payload, so it is rejected for a `deref` variant.
//!
//! ```
//! use std::rc::Rc;
//!
//! #[whichever::dispatchable]
//! pub trait Area {
//!     fn area(&self) -> u32;
//! }
//!
//! pub struct Square(pub u32);
//!
//! impl Area for Square {
//!     fn area(&self) -> u32 {
//!         self.0 * self.0
//!     }
//! }
//!
//! #[whichever::dispatch(Area)]
//! pub enum Shape<'a> {
//!     #[dispatch(deref)]
//!     Boxed(Box<Square>),
//!     #[dispatch(deref)]
//!     Shared(Rc<Square>),
//!     #[dispatch(deref)]
//!     Borrowed(&'a Square),
//!     Plain(Square),
//! }
//!
//! fn main() {
//!     let square = Square(3);
//!     assert_eq!(Shape::Boxed(Box::new(Square(2))).area(), 4);
//!     assert_eq!(Shape::Borrowed(&square).area(), 9);
//! }
//! ```
//!
//! # Inherent methods
//!
//! An enum often needs methods of its own that only some variants can
//! answer. `#[dispatch]` on an inherent `impl` block of a dispatch enum
//! forwards each method marked `#[dispatch(..)]` to the named variants'
//! method of the same name, and the body is what every other variant
//! returns. `*` names every variant, and the method then has no body:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Shape {
//!     fn area(&self) -> u32;
//! }
//!
//! pub struct Circle(pub u32);
//! pub struct Square(pub u32);
//! # impl Shape for Circle { fn area(&self) -> u32 { 3 * self.0 * self.0 } }
//! # impl Shape for Square { fn area(&self) -> u32 { self.0 * self.0 } }
//!
//! impl Circle {
//!     pub fn radius(&self) -> u32 {
//!         self.0
//!     }
//!     pub fn describe(&self) -> String {
//!         format!("circle {}", self.0)
//!     }
//! }
//!
//! impl Square {
//!     pub fn describe(&self) -> String {
//!         format!("square {}", self.0)
//!     }
//! }
//!
//! #[whichever::dispatch(Shape)]
//! pub enum AnyShape {
//!     Circle(Circle),
//!     Square(Square),
//! }
//!
//! #[whichever::dispatch]
//! impl AnyShape {
//!     /// Only circles have a radius.
//!     #[dispatch(Circle, with = Some)]
//!     pub fn radius(&self) -> Option<u32> {
//!         None
//!     }
//!
//!     #[dispatch(*)]
//!     pub fn describe(&self) -> String;
//! }
//!
//! fn main() {
//!     let square: AnyShape = Square(2).into();
//!     assert_eq!(square.radius(), None);
//!     assert_eq!(square.describe(), "square 2");
//! }
//! ```
//!
//! `with = path` wraps each forwarded result in a call to `path` (`Some`
//! above), `call = name` forwards to a payload method of another name, and
//! methods without the attribute are left alone. `async` methods and
//! methods returning `impl Future` are awaited. A method returning `Self`,
//! `Option<Self>` or `Result<Self, E>` wraps the payload's result back into
//! the variant, as trait dispatch does; with `with`, the payload returns
//! the plain value and `with` lifts the wrapped variant, so
//! `#[dispatch(Circle, with = Some)]` on `fn grown(&self) -> Option<Self>`
//! calls a `Circle::grown` that returns a `Circle`. The enum keeps its
//! definition in a crate-local macro for this, so the `impl` block has to be
//! in the enum's crate, which inherent impls are anyway. A bare `#[dispatch]`
//! on an enum registers it without dispatching a trait.
//!
//! # Traits from `std` and other crates
//!
//! A trait that is not `#[dispatchable]`, because another crate owns it,
//! is restated once with `remote` naming the real trait. Nothing is emitted
//! for the restatement but the carrier macro, and dispatching it implements
//! the real trait, forwarding exactly the methods restated:
//!
//! ```
//! use std::io::{self, Cursor};
//!
//! #[whichever::dispatchable(remote = std::io::Write)]
//! trait Write {
//!     fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
//!     fn flush(&mut self) -> io::Result<()>;
//!     fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
//! }
//!
//! #[whichever::dispatch(Write)]
//! pub enum Sink {
//!     Memory(Vec<u8>),
//!     Cursor(Cursor<Vec<u8>>),
//! }
//!
//! fn main() -> io::Result<()> {
//!     let mut sink: Sink = Vec::new().into();
//!     io::Write::write_all(&mut sink, b"hello")?;
//!     assert!(matches!(sink, Sink::Memory(bytes) if bytes == b"hello"));
//!     Ok(())
//! }
//! ```
//!
//! The restatements an enum most often needs ship in [`foreign`]:
//! `Display`, `Debug`, `Error`, `PartialEq`, `Iterator`, `Read`, `Write`,
//! `BufRead` and `Seek`. `#[dispatch(whichever::foreign::Display)]` on an
//! error enum replaces the hand-written `match`, and a variant holding a
//! `String` stays out of the `From` impls with `#[dispatch(from = false)]`.
//!
//! # Methods the enum implements itself
//!
//! A trait method without a receiver has no variant to dispatch on, and
//! sometimes the enum's answer differs from every payload's. `#[dispatch]`
//! on a trait impl block holding just those items forwards the rest of the
//! trait around them:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Handler: Sized {
//!     fn from_any(handler: AnyHandler) -> Option<Self>;
//!     fn is_attached(&self) -> bool;
//!     fn describe(&self) -> String {
//!         format!("attached: {}", self.is_attached())
//!     }
//! }
//!
//! pub struct Text;
//! pub struct Map;
//! # impl Handler for Text {
//! #     fn from_any(handler: AnyHandler) -> Option<Self> {
//! #         match handler { AnyHandler::Text(text) => Some(text), _ => None }
//! #     }
//! #     fn is_attached(&self) -> bool { true }
//! # }
//! # impl Handler for Map {
//! #     fn from_any(handler: AnyHandler) -> Option<Self> {
//! #         match handler { AnyHandler::Map(map) => Some(map), _ => None }
//! #     }
//! #     fn is_attached(&self) -> bool { false }
//! #     fn describe(&self) -> String { String::from("a map") }
//! # }
//!
//! #[whichever::dispatch]
//! pub enum AnyHandler {
//!     Text(Text),
//!     Map(Map),
//! }
//!
//! #[whichever::dispatch]
//! impl Handler for AnyHandler {
//!     fn from_any(handler: AnyHandler) -> Option<Self> {
//!         Some(handler)
//!     }
//! }
//!
//! fn main() {
//!     let map = AnyHandler::Map(Map);
//!     assert_eq!(map.describe(), "a map");
//!     assert!(AnyHandler::from_any(map).is_some());
//! }
//! ```
//!
//! The enum does not name that trait in its own attribute, since the block
//! is the impl. The trait's options, `Name = Type` and `mismatch`, go on
//! the block's attribute, and an associated type written in the block
//! counts as an override. A method written in the block replaces the
//! forwarding for every variant, provided methods included, which is also
//! the way to leave a payload's method out on purpose. The block's own
//! header, bounds included, becomes the impl's. Like inherent blocks, it
//! has to be in the enum's crate.
//!
//! # Wrapping the enum in a struct
//!
//! A dispatch enum is often private behind a public struct that forwards
//! the same trait. `#[dispatch]` on the struct forwards to its one field,
//! or to the field marked `#[dispatch]`, provided methods included, so a
//! payload's override reaches the public type:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Shape {
//!     fn area(&self) -> u32;
//!     fn name(&self) -> &'static str {
//!         "shape"
//!     }
//! }
//!
//! pub struct Square(pub u32);
//! # impl Shape for Square {
//! #     fn area(&self) -> u32 { self.0 * self.0 }
//! #     fn name(&self) -> &'static str { "square" }
//! # }
//!
//! #[whichever::dispatch(Shape)]
//! enum AnyShape {
//!     Square(Square),
//! }
//!
//! #[whichever::dispatch(Shape)]
//! pub struct Labelled {
//!     #[dispatch]
//!     shape: AnyShape,
//!     label: &'static str,
//! }
//!
//! fn main() {
//!     let labelled = Labelled { shape: Square(3).into(), label: "a" };
//!     assert_eq!((labelled.area(), labelled.name()), (9, "square"));
//! }
//! ```
//!
//! `#[dispatch(deref)]` on the field reaches through a `Box` or another
//! pointer. A method returning `Self` is rebuilt only around a struct with
//! no other field; otherwise it goes in a partial impl block, which works on
//! a struct as on an enum. A struct gets no `From` impls, and its `Self`
//! arguments need no `mismatch`.
//!
//! # Across crates
//!
//! `#[dispatchable]` emits the trait unchanged plus its carrier macro, a
//! `#[doc(hidden)]` `macro_rules!` holding the trait's tokens, re-exported
//! under the trait's own name (macros live in their own namespace, so
//! `use shapes::Shape;` imports both). For a `pub` trait the macro is
//! exported at the crate root, which is the only way another crate can
//! reach it; a trait of any other visibility keeps it in its module.
//! `#[dispatch(Shape)]` invokes that macro, which hands the trait and the
//! enum to the implementation macro, a proc macro that writes the impl.
//! Nothing is cached between expansions, so the two attributes work across
//! crates,
//! in any order, and under rust-analyzer. Both crates depend on
//! `whichever`; a renamed dependency is fine.
//!
//! The types named in the trait's signatures are written into the enum's
//! module, so they must be in scope there, exactly as for a hand-written
//! impl, and nameable from there: a private alias or type in a signature
//! limits the trait to its own crate. Paths starting with `crate::` are
//! rewritten to the trait's crate; `super::` and `self::` paths cannot be
//! and must not appear in a dispatchable trait's signatures.
//!
//! # Requirements and limits
//!
//! - Every variant a trait is dispatched to holds exactly one unnamed
//!   field, `Name(Payload)`, and every payload implements the trait. A
//!   variant of another shape is reported under its own `#[cfg]`, so a
//!   variant compiled out is no error, and an enum that only forwards
//!   inherent methods may hold such variants, which take the method's body.
//!   A payload type used by two variants gets no `From` impl.
//! - A method without a receiver needs a default body, which the enum uses,
//!   or an implementation in a `#[dispatch]` impl block.
//! - Return-position `impl Trait` cannot be dispatched by a `match`: every
//!   variant would return a different opaque type. `impl Future<Output = T>`
//!   is the exception; it is awaited like an `async fn`.
//! - Generic associated types are not supported.
//! - Receivers are `self`, `&self`, `&mut self`, `self: Box<Self>` on
//!   `deref` variants holding a `Box`, and `self: Pin<&Self>` or
//!   `self: Pin<&mut Self>` when every payload is `Unpin`. `Rc<Self>`,
//!   `Arc<Self>` and `Pin<Box<Self>>` would need a refcount or an
//!   allocation, so they are not.
//! - An associated const of a type other than an integer, `bool`, `char` or
//!   `&str`, or on a generic enum, is taken from the first variant without
//!   a check.
//! - One `macro_rules!` may emit a `#[dispatchable]` trait in several modules
//!   when at least one token of the trait, its name or a method's, comes
//!   from the invocation. A trait written out in full in the macro's body
//!   gets the same carrier macro name at every invocation, and rustc reports
//!   it as defined multiple times, unless each invocation passes its own
//!   `#[dispatchable(name = ..)]`.
//!
//! "cannot find macro `Shape` in this scope" at the enum means the trait is
//! not marked `#[dispatchable]`, or the trait (which carries the macro) is
//! not imported where the enum is. A `std::`, `core::` or `alloc::` path in
//! `#[dispatch]` is reported directly, naming the [`foreign`] declaration
//! or the `remote` restatement to use instead.
//!
//! The crate is `no_std`: the generated code names only `core` items. The
//! minimum supported Rust version is 1.88.
#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub use whichever_macros::{dispatch, dispatchable};

pub mod foreign;

#[doc(hidden)]
pub mod __private {
    pub use whichever_macros::{implement, implement_inherent};
}
