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
//! and no conversion in the arms unless an option asks for one. Three more
//! things are in the expansion: the impl is marked
//! `#[automatically_derived]`, which tells Clippy and coverage tools what
//! it is; the arms' bindings are named `__inner`, `__same0` and the like,
//! with `macro_rules`-style hygiene, so they never collide with a name the
//! trait wrote; and a `const _` block beside the impl holds the agreement
//! checks, which compile to nothing.
//!
//! # Options
//!
//! All options follow the trait path in `#[dispatch(..)]`:
//!
//! - `Name = Type` fixes the associated type `Name` instead of taking it
//!   from the first variant. A method returning `Self::Name` is then
//!   converted with [`Into`], one returning `Result<_, Self::Name>` with
//!   `map_err(Into::into)`, and `Option<Self::Name>`, `Result<Self::Name, E>`
//!   and `Result<Self, Self::Name>` are mapped the same way, so variants
//!   whose own type converts into the named one can share the enum. See
//!   [associated types](#associated-types).
//! - `Name = value` fixes the associated const `Name` instead of taking it
//!   from the first variant, for a const that differs per payload by
//!   design, such as a kind discriminator.
//! - `mismatch(method = expr, ..)` gives the result of `method` when its
//!   `Self` arguments hold a different variant than `self`. The trait author
//!   can decide this once instead, with `#[dispatch(mismatch = expr)]` on
//!   the method. See [`Self` arguments](#self-arguments).
//! - `from = false` suppresses the `From<Payload>` impls. A payload type
//!   held by two variants gets none either way, and a payload whose `From`
//!   is written by hand, or that a derive supplies, stays out with
//!   `#[dispatch(from = false)]` on its variant.
//! - `try_from = true` adds `TryFrom<Enum> for Payload` for every payload
//!   type, with the enum itself as the error, so a mismatch hands the value
//!   back; `.unwrap()` on the result therefore needs the enum to be `Debug`.
//!   `try_from = ref` adds `TryFrom<&Enum> for &Payload` to that, and
//!   `try_from = ref_mut` adds `TryFrom<&mut Enum> for &mut Payload` too.
//! - `into = Target` adds `From<Enum> for Target`, each arm converting the
//!   payload through its own `Into<Target>`; the option may be repeated.
//!
//! Four options go on a variant. `#[dispatch(from = false)]` leaves the
//! variant out of the `From` and `TryFrom` impls, for a payload such as
//! `String` that should not convert into the enum, and
//! `#[dispatch(try_from = true)]` adds the `TryFrom` impl for that payload
//! alone. `#[dispatch(deref)]` calls the trait on what the payload
//! dereferences to, so `Box<T>`, `Rc<T>`, `Arc<T>`, `&T` and `&mut T`
//! payloads dispatch like a plain `T`, and `#[dispatch(via = route)]` on a
//! field of the payload or on what a method of it returns. See
//! [reaching the target](#reaching-the-target). A trait's own route,
//! `via(Variant = route, ..)` after the trait path, wins over the
//! variant's for that trait.
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
//! | generic method | Called with its type and const parameters spelled out when no argument fixes them. |
//! | `self: Box<Self>` | The enum is moved out of its box and a `deref` variant's own `Box<Payload>` is handed over; a plain variant is an error. |
//! | `self: Pin<&Self>`, `self: Pin<&mut Self>` | The enum is unpinned, matched, and the payload pinned again; every payload must be `Unpin`. |
//! | default body | Ignored: the method is forwarded like any other. |
//! | `async fn` | Each arm is awaited. |
//! | `-> impl Future<Output = T>` | The `match` sits in one `async move` block and each arm is awaited; the future's bounds, `Send` included, are checked against that block. On a struct the one arm returns the field's future as it is, unless its output is rewrapped or converted. |
//! | other `-> impl Trait` | An error on an enum, whose arms would each return a different opaque type; forwarded as it is on a struct, whose one arm returns one. |
//! | `unsafe fn` | Each call is wrapped in `unsafe { }`. |
//! | `unsafe trait` | An `unsafe impl`. |
//! | argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`; different variants give the `mismatch` expression. |
//! | return `Self` | Wrapped back into the same variant; `Option<Self>` and `Result<Self, E>` are mapped, under any path, `io::Result<Self>` included; a tuple holding `Self`, or an overridden associated type, is taken apart and built again around its converted elements. `Self` anywhere else in a return type, or a tuple under an `Option` or a `Result`, is an error. |
//! | return `Self::X` with `X` overridden | Converted with `Into`; inside `Option` or `Result` with `map(Into::into)`, and `Result<_, Self::X>` with `map_err(Into::into)`. |
//! | method without a receiver | Left to the trait's default body, or written in a `#[dispatch] impl Trait for Enum` block; an error when it has neither. |
//! | `#[cfg]` on a method or an associated item | Decided in the trait's crate: the carrier macro is written once per combination of the trait's distinct predicates, each under its own `cfg`, so the generated item exists exactly when the trait's does, in a build of the trait's crate. |
//! | `#[cfg]` on a variant | Carried onto the arm, and decided where the enum is. An associated type or const comes from the first variant without a `#[cfg]`. |
//! | `#[deprecated]` on a variant | Its arms and its conversions are written under `#[allow(deprecated)]`; the deprecation reaches the code that names the variant. |
//! | `#[deprecated]` on a method or an associated item | Forwarded under `#[allow(deprecated)]`, as a hand writes it; the deprecation reaches the callers through the trait's item. |
//! | `#[inline]` | Added to every generated method; a trait method's own `#[inline(..)]` is carried instead. |
//! | `#[allow(..)]` on a method or an associated item | Carried onto the generated item, which repeats the signature the lint was allowed on. `warn`, `deny` and `expect` stay the trait crate's, and so does an `#[allow]` another macro added. |
//!
//! # Generics
//!
//! A generic trait is dispatched at an instantiation named on the enum: a
//! concrete one, `#[dispatch(Encoder<Json>)]`, or the enum's own
//! parameters, `#[dispatch(GridTrait<T>)]` on `enum Grid<T>`; a
//! parameter with a default may be left out, `#[whichever::dispatch(Enc)]`
//! for `trait Enc<F = Json>`, as `impl Enc for A` leaves it out. Type,
//! lifetime and const parameters are substituted in every signature, and a
//! projection such as `F::Value` becomes `<Json as Format>::Value` when `F`
//! has exactly one bound; with several bounds the trait has to spell
//! `<F as Format>::Value` itself. The enum itself may be generic; its
//! parameters, bounds and `where` clause carry into the impl, and a payload
//! that is one of its type parameters, `Raw(Raw)` on `enum Lazy<Raw,
//! Parsed>` or `Box<T>` under `deref`, gives the impl the bound `Raw:
//! Trait` a hand writes; a struct's field the same.
//!
//! # `Self` arguments
//!
//! A method such as `fn congruent(&self, other: &Self) -> bool` compares two
//! values of the enum. The generated method matches both on the same variant
//! and calls the payload's method; for two different variants it returns the
//! expression given by `#[dispatch(mismatch = false)]` on the trait method,
//! or by `mismatch(congruent = false)` on the enum. Without one the expansion
//! fails with an error naming the method, unless the enum has one variant,
//! which has no second variant to meet and gets no fallback arm. Only `Self`, `&Self` and
//! `&mut Self` are accepted; a `Self` nested in another type
//! (`Option<&Self>`) is an error. An expression on the trait method or on
//! the enum may use the method's parameters, a `Self` argument included,
//! and `self` when the method takes `&self`; `&mut self` and `self` are
//! moved into the `match`. A path in an expression on the trait method
//! resolves where the enum is, as the signature types do, so a helper beside
//! the trait is spelled from `crate::`.
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
//! # Reaching the target
//!
//! An arm calls the trait on the payload. A route names the steps from the
//! payload to the value the trait is called on instead: `*` for what the
//! payload dereferences to, which `deref` spells, `.field` or `.0` for a
//! field, `.method()` for what a method returns, in any sequence. It is
//! written on a variant or a struct field for every trait, `#[dispatch(via
//! = .base)]`; as the marker on one field of a variant with several,
//! `Node { id: u32, #[dispatch] data: Data }`; on the trait, keyed by
//! variant, when the route differs per trait, `#[dispatch(Trait,
//! via(VarChar = .0))]`; or on an inherent method, for that method alone.
//! Where the route names the target's type, the call is `<Square as
//! Shape>::area(&inner.base)`; a field's or a method's type cannot be
//! named, and the call is `<_ as Shape>::area(&inner.base)`, qualified by
//! the trait alone. A route ending in a method call serves `&self` and
//! `self` methods, not `&mut self`; a method returning `Self` is reported
//! on a routed variant, `deref` included; a variant with other fields
//! beside the payload gets no `From` or `TryFrom`, and a target without a
//! nameable type takes no part in the agreement checks. A `deref` variant
//! holding a trait object, `Box<dyn Trait>` or `&dyn Trait`, is how an enum
//! holds an open set beside its sized payloads; a method the trait declares
//! with `where Self: Sized` cannot be called on it, and is reported in one
//! sentence, so a partial block leaves that method to the trait's default
//! with `#[dispatch(default)]`, as a hand does, and the rest is forwarded.
//!
//! A variant marked `#[dispatch(deref)]` holds its value behind a pointer or
//! a reference. The trait is called on `<Payload as Deref>::Target`, with
//! the receiver and any `Self` argument dereferenced to match: `&**inner`
//! for `&self`, `&mut **inner` for `&mut self` (the payload must implement
//! `DerefMut`), `*inner` for `self` (a `Box` moves out; a reference or a
//! shared pointer, which a value cannot be moved out of, is reported).
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
//! Each arm calls the payload's method by method-call syntax,
//! `inner.describe()`, so autoref, autoderef and inherent-before-trait
//! resolution apply as in hand-written code. `with = path` wraps each
//! forwarded result in a call to `path` (`Some` above), which may be a
//! generic function, called separately in each arm, so payloads whose
//! results differ can meet in one declared return type, and `with =
//! Into::into` rebuilds the enum through its `From` impls, which is the way
//! to return the enum at another lifetime; `call = name`
//! forwards to a payload method of another name; and methods without the
//! attribute are left alone. `async` methods and methods returning
//! `impl Future` are awaited. A method returning `Self`, `Option<Self>` or
//! `Result<Self, E>` wraps the payload's result back into the variant, as
//! trait dispatch does; with `with`, the payload returns the plain value and
//! `with` lifts the wrapped variant, so `#[dispatch(Circle, with = Some)]`
//! on `fn grown(&self) -> Option<Self>` calls a `Circle::grown` that returns
//! a `Circle`. `field = name` makes the arm read that field of the target,
//! by reference when the method returns one and by value otherwise; `value`
//! makes the arm the target itself, for `with` to wrap or the return type
//! to widen, `Some(inner)` as an `Option<&dyn Trait>`; and `via = route`
//! reaches the target for this method alone. A `Self` argument is the enum itself and is handed to the
//! payload's method as it is. A variant under `#[cfg]` may be named; where
//! it is compiled out, the method takes its body. The enum keeps its
//! definition in a crate-local macro for this, so the `impl` block has to be
//! in the enum's crate, which inherent impls are anyway. A bare `#[dispatch]`
//! on an enum registers it without dispatching a trait. A block on a
//! dispatch struct forwards the same way: `*` is the field marked bare, or
//! the only field, and a field's name is that field, marked or not; there a
//! `Self` argument is the other value's field, as in trait dispatch, and a
//! return-position `impl Trait` is forwarded as it is.
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
//! Write the signature types in full, as [`foreign`] does, or import them
//! where the enum is: nothing but the macro reads the restatement, so its
//! own imports count as unused. Bounds on a restated associated type may be
//! left out. A restatement of an `#[async_trait]` trait carries
//! `#[async_trait]` above `#[dispatchable(remote = ..)]`, as the trait
//! itself does.
//!
//! The restatements an enum most often needs ship in [`foreign`]:
//! `Display`, `Debug`, `Error`, `PartialEq`, `Iterator`, `Read`, `Write`,
//! `BufRead` and `Seek`. `#[dispatch(whichever::foreign::Display)]` on an
//! error enum replaces the hand-written `match`, and a variant holding a
//! `String` stays out of the `From` impls with `#[dispatch(from = false)]`.
//!
//! # Enums from other crates
//!
//! A trait of this crate may be forwarded over an enum another crate
//! defines, as a hand-written `impl LocalTrait for other::Either<L, R>`
//! may: restate the enum once, `remote` naming the real one, and dispatch
//! the restatement, `#[whichever::dispatch(remote = either::Either)]
//! #[whichever::dispatch(Describe)] enum Either<L, R> { Left(L), Right(R) }`.
//! Nothing is emitted for the restatement but its carrier and the impls,
//! written for the real path. A restated enum gets no conversions and no
//! inherent impl, which the orphan rules and the language refuse; a
//! partial impl block is written on the real enum, imported under the
//! restatement's name.
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
//! is the impl; naming it in both is reported on the block. The trait's
//! options, `Name = Type` and `mismatch`, go on the block's attribute, and
//! an associated type written in the block counts as an override. A generic
//! associated type is declared here, since no first variant can supply one,
//! `type Item<'a> = AnyItem<'a> where Self: 'a;`, and each payload's own
//! converts into it through `Into`, as for any override. A method written
//! in the block replaces the forwarding for every variant, provided methods
//! included, which is also the way to leave a payload's method out on
//! purpose; one written under `#[cfg]` replaces it where the `cfg` holds
//! and is forwarded where it does not, as a hand writes the two arms. A
//! method in the block may name the variants it forwards to, as an
//! inherent method does, `#[dispatch(Wep, Wpa)] fn fmt(..) { .. }`, and
//! its body is then the arm for every other variant, a unit variant or two
//! different variants of a `Self` argument included; `#[dispatch(default)]`
//! on a bodiless method leaves it to the trait's default, which over a
//! restatement is the real trait's. The
//! block's own header, bounds included, becomes the impl's,
//! and may name the enum's parameters as it likes, `impl<T, Ty> Get<T> for
//! ImmOrNode<Ty>`. The arms name the payload types as the enum spells them,
//! so a block in another module needs them in scope there; the converse
//! holds too, so when the trait's signature types are not in scope where
//! the enum is, an empty block in a module where they are is the place to
//! write the impl. Outer attributes
//! on the block are carried onto the impl: `#[typetag::serde(name = "..")]`
//! goes there, with `#[whichever::dispatchable]` above `#[typetag::serde]`
//! on the trait, so that typetag's own methods are the block's and never
//! forwarded. A method written in the block that still needs the payloads'
//! answer calls an inherent `#[dispatch(*, call = name)]` method beside it.
//! Like inherent blocks, it has to be in the enum's crate.
//!
//! The block's header is the impl's header, so an empty block is how the
//! impl gets a header the enum's attribute cannot write: generics the enum
//! does not have, `impl<F: Format> Encoder<F> for AnyEncoder {}` for a
//! trait generic over its own parameter or `impl<'r> Renderable<'r> for
//! Node {}` for a lifetime; one instantiation of a generic enum; or a bound
//! on a type parameter beyond the `Param: Trait` the impl gets on its own.
//!
//! # Forwarding from a struct field
//!
//! A dispatch enum is often private behind a public struct that forwards
//! the same trait. `#[dispatch]` on the struct forwards to its one field,
//! or to the field marked `#[dispatch]`, provided methods included, so a
//! payload's override reaches the public type; the field may be any
//! implementor, so the same attribute replaces a field-delegation derive:
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
//! A field may name the traits it is the target of, `#[dispatch(Rom)] rom:
//! Cartridge` beside `#[dispatch(Interrupt)] signals: Signals`, and the
//! field marked bare takes every trait not named elsewhere; the struct's
//! attribute still names every trait it dispatches. A field names a trait as
//! the struct's attribute spells it, arguments included, so two
//! instantiations of one generic trait, or two traits of one name from two
//! modules, go to two fields.
//! `#[dispatch(deref)]` on the field reaches through a `Box` or another
//! pointer. A method returning `Self` is rebuilt only around a struct with
//! no other field; otherwise it goes in a partial impl block, which works on
//! a struct as on an enum. A struct's `match` has one arm, so it forwards a
//! return-position `impl Trait` as it is, which an enum cannot. A struct
//! gets no `From` impls, and its `Self` arguments need no `mismatch`: two
//! values of it always hold the same field, and the argument is unwrapped
//! to it.
//!
//! # Across crates
//!
//! `#[dispatchable]` emits the trait unchanged plus its carrier macro, a
//! `#[doc(hidden)]` `macro_rules!` holding the trait's signatures, re-exported
//! under the trait's own name (macros live in their own namespace, so
//! `use shapes::Shape;` imports both). For a `pub` trait the macro is
//! exported at the crate root, which is the only way another crate can
//! reach it; a trait of any other visibility keeps it in its module.
//! `#[dispatch(Shape)]` invokes that macro, which hands the trait and the
//! enum to the implementation macro, a proc macro that writes the impl.
//! Nothing is cached between expansions, so the two attributes work across
//! crates,
//! in any order, and under rust-analyzer. Both crates depend on
//! `whichever`, before 1.0 on the same minor, since the carrier is read by
//! the enum crate's `whichever`: a newer one reads every older carrier, and
//! an older one reports a newer carrier in one sentence. A renamed
//! dependency is fine.
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
//!   field, `Name(Payload)`, or marks the field to forward to with
//!   `#[dispatch]`, and every target implements the trait. A
//!   variant of another shape is reported under its own `#[cfg]`, so a
//!   variant compiled out is no error, and an enum that only forwards
//!   inherent methods may hold such variants, which take the method's body.
//!   A payload type used by two variants gets no `From` impl.
//! - A method without a receiver needs a default body, which the enum uses,
//!   or an implementation in a `#[dispatch]` impl block.
//! - An enum cannot dispatch a return-position `impl Trait`, at any depth
//!   of the return type: every variant would return a different opaque
//!   type. `impl Future<Output = T>` is the exception; it is awaited like an
//!   `async fn`. A struct forwards any.
//! - A method that requires `Self: Sized` cannot be forwarded to a `deref`
//!   variant holding a trait object; a partial block leaves it to the trait's
//!   default with `#[dispatch(default)]`.
//! - A generic associated type is declared by the enum, in a partial impl
//!   block; it cannot come from the first variant.
//! - A projection through a trait parameter with several bounds,
//!   `F::Value` with `F: Format + Clone`, cannot be qualified at the enum;
//!   the trait writes `<F as Format>::Value`.
//! - An associated type or const a partial impl block declares under
//!   `#[cfg]` is an error: the forwarded methods convert into the type in
//!   every build. Put the whole block under the `cfg`, with a second block
//!   for the other builds.
//! - A lint attribute on the enum does not reach the impls written next to
//!   it, as it would not reach a hand-written one; `#[allow]` goes on the
//!   trait method, on a partial impl block, or on the module.
//! - A private glob import that brings the trait's own name back into the
//!   trait's module, `use crate::prelude::*;` at a crate root whose prelude
//!   re-exports a trait declared there, makes the carrier's re-export
//!   ambiguous to rustc (`ambiguous_import_visibilities`, an error under
//!   `#![deny(future_incompatible)]`). Declare the trait in a module of its
//!   own and re-export it, or import the prelude's items by name there.
//! - Receivers are `self`, `&self`, `&mut self`, `self: Box<Self>` on
//!   `deref` variants holding a `Box`, and `self: Pin<&Self>` or
//!   `self: Pin<&mut Self>` when every payload is `Unpin`. `Rc<Self>`,
//!   `Arc<Self>` and `Pin<Box<Self>>` would need a refcount or an
//!   allocation, so they are not.
//! - An associated const of a type other than an integer, `bool`, `char` or
//!   `&str`, or on a generic enum, is taken from the first variant without
//!   a check.
//! - A trait whose items carry more than four distinct `#[cfg]` predicates
//!   is reported on the fifth: the carrier macro is written once per
//!   combination of them. Combine the predicates, or move the items under
//!   the fifth to a trait of their own.
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
//! or the `remote` restatement to use instead. "could not find `Trait` in
//! `other`" on `#[dispatch(other::Trait)]` means the same for a trait of
//! another crate: restate it with `remote`. The same words on a partial
//! block over another crate's enum mean the enum has no
//! `#[whichever::dispatch]`, which only its own crate can add. With
//! `#[async_trait]`, `#[dispatchable]` goes below it; with
//! `#[typetag::serde]`, above it; either order the other way round is
//! reported.
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
    pub use whichever_macros::{implement, implement_block};
}
