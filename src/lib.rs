//! Implements a trait for an enum by forwarding to whichever variant it holds.
//!
//! You have a trait and a few types that implement it, and you want to hold
//! "one of these". An enum is the cheapest way to do that: no `Box`, no
//! vtable, no allocation, and the optimiser can see into every arm.
//!
//! The catch is the boilerplate. The enum needs its own impl of the trait,
//! and every method in it is the same `match`, passing the call on to
//! whichever variant is inside.
//!
//! `whichever` writes that impl for you, with two attributes:
//!
//! - [`#[dispatchable]`](macro@dispatchable) goes on the trait, in the crate
//!   that owns it.
//! - [`#[dispatch(Trait)]`](macro@dispatch) goes on an enum whose variants
//!   each hold one value that implements the trait, in any crate.
//!
//! The enum then implements the trait with one `match` per method. Each arm
//! is a fully qualified call on the variant's payload, the value the variant
//! holds. You also get one `From<Payload>` impl per variant. Add the crate
//! with `cargo add whichever`, in the crate that owns the trait and in the
//! crate that owns the enum.
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
//! An enum with `#[dispatch]` on it is a *dispatch enum*. Passing a method
//! call on to the payload is *forwarding*. The rest of this page is a
//! [tour of the features](#a-tour-of-the-features), then a
//! [reference](#reference) you can jump into from the sidebar. New to the
//! crate? [The tutorial](docs::tutorial) takes it one step at a time.
//!
//! # What is generated
//!
//! For the enum above, `#[dispatch(Shape)]` writes the impl you would write
//! by hand, plus `#[inline]` and the conversions:
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
//! What to notice:
//!
//! - **One `match` per method**, with one arm per variant.
//! - **Every call is fully qualified**: `<Square as Shape>::area(inner)`,
//!   not `inner.area()`. So an inherent method on a payload with the same
//!   name as a trait method never takes over.
//! - **Provided methods are forwarded too.** A provided method is a trait
//!   method with a default body, such as `name`. The enum still calls each
//!   payload's `name`, so a variant's override is never hidden by the
//!   trait's default at the enum level. That is why `square.name()` above
//!   returns `"square"`.
//! - **Every method is `#[inline]`**, so the optimiser can see through the
//!   `match` into the payload's method, even across crates.
//! - **Nothing else is added**: no trait object, no closure, and no
//!   conversion in the arms unless an option asks for one.
//!
//! Run `cargo expand` and you see a few more things. None of them adds any
//! work at run time:
//!
//! - The impl is marked `#[automatically_derived]`. This tells Clippy and
//!   coverage tools that the code is generated.
//! - The bindings in the arms are called `__inner`, `__same0` and the like,
//!   not `inner`. They have `macro_rules`-style hygiene, so they never
//!   collide with a name the trait uses.
//! - A `const _` block next to the impl holds the agreement checks:
//!   compile-time checks that every variant has the same associated types
//!   and consts. They compile to nothing.
//!
//! # A tour of the features
//!
//! Each section shows the everyday use of one feature first, then the
//! details. You can stop reading a section early and still be right.
//!
//! ## Conversions
//!
//! By default the enum gets `From<Payload>` for every variant, which is why
//! `Square(3).into()` works above. Options after the trait path add more
//! conversions, or take them away:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Shape {
//!     fn area(&self) -> u32;
//! }
//!
//! #[derive(Debug, PartialEq)]
//! pub struct Circle(pub u32);
//! #[derive(Debug, PartialEq)]
//! pub struct Square(pub u32);
//! # impl Shape for Circle { fn area(&self) -> u32 { 3 * self.0 * self.0 } }
//! # impl Shape for Square { fn area(&self) -> u32 { self.0 * self.0 } }
//!
//! #[derive(Debug)]
//! #[whichever::dispatch(Shape, try_from = ref)]
//! pub enum AnyShape {
//!     Circle(Circle),
//!     Square(Square),
//! }
//!
//! fn main() {
//!     let shape = AnyShape::from(Circle(2));
//!
//!     // By reference: the payload, when the variant matches.
//!     assert_eq!(<&Circle>::try_from(&shape).unwrap(), &Circle(2));
//!     assert!(<&Square>::try_from(&shape).is_err());
//!
//!     // By value: a failed conversion hands the enum back.
//!     let back = Square::try_from(shape).unwrap_err();
//!     assert!(matches!(back, AnyShape::Circle(Circle(2))));
//! }
//! ```
//!
//! The options on the enum:
//!
//! - **`from = false`** turns the `From<Payload>` impls off.
//! - **`try_from = true`** adds `TryFrom<Enum> for Payload` for every payload
//!   type. The error type is the enum itself, so a value that holds another
//!   variant is handed back. That is why `.unwrap()` on the result needs the
//!   enum to be `Debug`.
//! - **`try_from = ref`** adds `TryFrom<&Enum> for &Payload` as well, and
//!   **`try_from = ref_mut`** adds `TryFrom<&mut Enum> for &mut Payload` on
//!   top of both. Each by-reference impl hands the reference back as the
//!   error.
//! - **`into = Target`** adds `From<Enum> for Target`. Each arm converts its
//!   payload through the payload's own `Into<Target>`. You can repeat the
//!   option for several targets. Every variant has to hold a payload and
//!   nothing beside it, because the impl has to convert every variant; a
//!   variant under `#[dispatch(from = false)]` is converted like the rest.
//!
//! Two options go on a variant instead, in `#[dispatch(..)]` above it:
//!
//! - **`#[dispatch(from = false)]`** leaves that variant out of the `From`
//!   and `TryFrom` impls. Use it for a payload such as `String` that should
//!   not convert into the enum. Use it too for a payload whose `From` you
//!   write by hand or a derive supplies, because a second impl of the same
//!   `From` is a conflict.
//! - **`#[dispatch(try_from = true)]`** adds the `TryFrom` impl for that
//!   variant's payload alone.
//!
//! The `From` impls reach one level: an enum held by another gives the outer
//! enum `From<Inner>`, not the inner enum's payloads. One blanket impl by
//! hand reaches them, `impl<T> From<T> for Outer where Inner: From<T>`,
//! calling `Outer::Inner(Inner::from(value))`, with `#[dispatch(from =
//! false)]` on the variant, whose own `From<Inner>` the blanket covers.
//!
//! A payload type that two variants hold gets no `From` either way: there
//! would be two impls of the same `From`, one per variant. Neither does a
//! payload that is a type parameter of the enum, because `From<T>` would
//! overlap the other variants' impls; the error asks for `from = false`, on
//! the enum or on that variant. A payload that is an associated type of a
//! type parameter, `Out(S::Out)`, may be any type, the enum included, so
//! its `From` would overlap `From<T> for T`; you get an error asking for
//! `#[dispatch(from = false)]` on it. A projection from a concrete type,
//! `<Fixed as Source>::Out`, gets its `From`.
//!
//! Some variants and types get no conversions at all: a variant with other
//! fields beside the payload
//! ([routes](#reaching-the-target-pointers-fields-and-methods)), a
//! [struct](#forwarding-from-a-struct-field), and an
//! [enum from another crate](#enums-from-other-crates). When an enum
//! dispatches several traits, the conversions are still written once; see
//! [several traits](#several-traits-on-one-enum).
//!
//! ## Associated types and consts
//!
//! Without an override, the enum takes each associated type and const from
//! its first variant. A hidden check makes sure every other variant uses the
//! same one, and you get the error on the variant that differs. This is the
//! *agreement check*.
//!
//! When the variants differ on purpose, fix the item on the enum with an
//! *override*. With `Error = StorageError`, the enum's `Error` is
//! `StorageError`, and each variant's own error type only has to implement
//! `Into<StorageError>`:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Storage {
//!     const BLOCK_SIZE: usize;
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
//!     const BLOCK_SIZE: usize = 4096;
//!     type Error = StorageError;
//!     fn load(&self, key: &str) -> Result<u32, StorageError> {
//!         Ok(key.len() as u32)
//!     }
//! }
//!
//! impl Storage for Disk {
//!     const BLOCK_SIZE: usize = 4096;
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
//!     assert_eq!(AnyStorage::BLOCK_SIZE, 4096);
//!     let disk: AnyStorage = Disk.into();
//!     assert_eq!(disk.load("answer"), Err(StorageError("offline".to_owned())));
//! }
//! ```
//!
//! `BLOCK_SIZE` comes from `Memory`, the first variant, and is checked
//! against `Disk`. `Disk`'s `&'static str` error is converted on the way
//! out.
//!
//! **`Name = Type`** fixes the associated type `Name`. So variants whose own
//! type converts into it can share the enum. What a method returns is
//! converted:
//!
//! - `Self::Name` with [`Into`].
//! - `Result<_, Self::Name>` with `map_err(Into::into)`.
//! - `Option<Self::Name>`, `Result<Self::Name, E>` and
//!   `Result<Self, Self::Name>` are mapped the same way, with
//!   `map(Into::into)` inside the `Option` or the `Result`.
//! - A tuple holding `Self::Name` is taken apart and built again around its
//!   converted elements.
//!
//! An argument is not converted. An argument that uses the type is handed to
//! the payload as it is, and so is a closure over it. So a method taking
//! `Self::Name` compiles only where every payload's own `Name` already is the
//! override's type. See [the limit](#associated-items) for what to do
//! instead.
//!
//! **`Name = value`** fixes the associated const `Name`. Use it for a const
//! that differs per payload by design, such as a kind discriminator:
//! `#[whichever::dispatch(Proof, KIND = "any")]`.
//!
//! **Which consts are checked.** A const is checked against every variant
//! at compile time when both of these are true:
//!
//! - its type is an integer, a float, `bool`, `char` or `&str`;
//! - the enum has no type or const parameter (lifetime parameters do not
//!   matter here).
//!
//! Every variant that disagrees gets its own error. Any other const is taken
//! from the first variant without a check. A variant reached through a field
//! or a method is left out of the check: its const is not compared.
//!
//! A generic associated type cannot come from the first variant, and no
//! option gives one. You declare it in a
//! [partial impl block](#partial-impl-blocks-write-some-methods-yourself).
//! A struct takes it from its field's type instead, where it can name that
//! type.
//!
//! ## `Self` arguments and `mismatch`
//!
//! A method such as `fn congruent(&self, other: &Self) -> bool` compares two
//! values of the enum. The generated method matches `self` and `other` on
//! the same variant and calls the payload's method. When they hold different
//! variants, there is no payload method to call. The method then returns the
//! *mismatch* expression you give it:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Shape {
//!     fn area(&self) -> u32;
//!     /// Two different kinds of shape are never congruent.
//!     #[dispatch(mismatch = false)]
//!     fn congruent(&self, other: &Self) -> bool;
//! }
//!
//! pub struct Circle(pub u32);
//! pub struct Square(pub u32);
//! # impl Shape for Circle {
//! #     fn area(&self) -> u32 { 3 * self.0 * self.0 }
//! #     fn congruent(&self, other: &Circle) -> bool { self.0 == other.0 }
//! # }
//!
//! impl Shape for Square {
//!     fn area(&self) -> u32 {
//!         self.0 * self.0
//!     }
//!     fn congruent(&self, other: &Square) -> bool {
//!         self.0 == other.0
//!     }
//! }
//!
//! #[whichever::dispatch(Shape)]
//! pub enum AnyShape {
//!     Circle(Circle),
//!     Square(Square),
//! }
//!
//! fn main() {
//!     let square = AnyShape::from(Square(2));
//!     assert!(square.congruent(&AnyShape::from(Square(2))));
//!     assert!(!square.congruent(&AnyShape::from(Square(3))));
//!     // Different variants: the mismatch expression, `false`.
//!     assert!(!square.congruent(&AnyShape::from(Circle(2))));
//! }
//! ```
//!
//! You write the mismatch in one of two places:
//!
//! - **On the trait method**, `#[dispatch(mismatch = false)]`, as above. The
//!   trait's author decides it once.
//! - **On the enum**, `mismatch(congruent = false)` after the trait path,
//!   for each method by name. Use it when the trait is not yours.
//!
//! Without either, you get an error naming the method. An enum with one
//! variant is the exception: it has no second variant to meet, and gets no
//! mismatch arm.
//!
//! The fine print:
//!
//! - Only `Self`, `&Self` and `&mut Self` arguments are accepted. A `Self`
//!   nested in another type, such as `Option<&Self>`, is an error.
//! - The expression may use the method's parameters, a `Self` argument
//!   included, and `self` when the method takes `&self`. Under `&mut self`
//!   and `self`, the receiver is moved into the `match`, so the expression
//!   cannot use it.
//! - A path in an expression on the trait method resolves where the enum
//!   is, as the signature types do. So a helper next to the trait is spelled
//!   from `crate::`.
//! - A method the trait declares under a `#[cfg]` takes its mismatch on the
//!   trait method. On the enum, the option has to refer to a method that
//!   exists in every build.
//! - A [dispatch struct](#forwarding-from-a-struct-field) needs no
//!   mismatch.
//!
//! ## `Self` in return types
//!
//! A method that returns `Self` gets the payload's result wrapped back into
//! the same variant. So do `Option<Self>` and `Result<Self, E>`:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Shape: Sized {
//!     fn side(&self) -> u32;
//!     fn scaled(&self, factor: u32) -> Self;
//!     fn halved(&self) -> Option<Self>;
//! }
//!
//! pub struct Square(pub u32);
//! # pub struct Circle(pub u32);
//! # impl Shape for Circle {
//! #     fn side(&self) -> u32 { self.0 }
//! #     fn scaled(&self, factor: u32) -> Self { Circle(self.0 * factor) }
//! #     fn halved(&self) -> Option<Self> { None }
//! # }
//!
//! impl Shape for Square {
//!     fn side(&self) -> u32 {
//!         self.0
//!     }
//!     fn scaled(&self, factor: u32) -> Self {
//!         Square(self.0 * factor)
//!     }
//!     fn halved(&self) -> Option<Self> {
//!         (self.0 % 2 == 0).then(|| Square(self.0 / 2))
//!     }
//! }
//!
//! #[whichever::dispatch(Shape)]
//! pub enum AnyShape {
//!     Circle(Circle),
//!     Square(Square),
//! }
//!
//! fn main() {
//!     let big = AnyShape::from(Square(2)).scaled(3);
//!     assert!(matches!(big, AnyShape::Square(Square(6))));
//!     assert!(matches!(big.halved(), Some(AnyShape::Square(Square(3)))));
//!     assert!(AnyShape::from(Square(3)).halved().is_none());
//! }
//! ```
//!
//! The details:
//!
//! - `Option<Self>` and `Result<Self, E>` are found under any path, so
//!   `io::Result<Self>` works.
//! - A tuple holding `Self` or an overridden associated type, such as
//!   `(Self, u32)`, is taken apart and built again around its converted
//!   elements.
//!
//! `Self` anywhere else in a return type is an error, because the rewrap
//! would need a closure:
//!
//! - `Self` inside another type, such as a combinator's
//!   `fn buffered(self) -> Buffered<Self>`;
//! - a tuple under an `Option` or a `Result`, such as `Option<(Self, u32)>`;
//! - `Option<Self>` or `Result<Self, E>` on a
//!   [struct](#forwarding-from-a-struct-field) whose one field is named, or
//!   on a variant whose one field is named and marked.
//!
//! If such a method is a provided method, leave it to the trait's default
//! with `#[dispatch(default)]` in a
//! [partial impl block](#partial-impl-blocks-write-some-methods-yourself).
//! A method returning `Self` also cannot use a
//! [routed variant](#what-a-route-cannot-do).
//!
//! ## Several traits on one enum
//!
//! Repeat the attribute, one trait per attribute. A supertrait is one more
//! attribute:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Shape {
//!     fn area(&self) -> u32;
//! }
//!
//! #[whichever::dispatchable]
//! pub trait Drawable: Shape {
//!     fn draw(&self) -> String;
//! }
//!
//! pub struct Square(pub u32);
//! # pub struct Circle(pub u32);
//! # impl Shape for Circle { fn area(&self) -> u32 { 3 * self.0 * self.0 } }
//! # impl Drawable for Circle { fn draw(&self) -> String { format!("circle {}", self.0) } }
//!
//! impl Shape for Square {
//!     fn area(&self) -> u32 {
//!         self.0 * self.0
//!     }
//! }
//!
//! impl Drawable for Square {
//!     fn draw(&self) -> String {
//!         format!("square of area {}", self.area())
//!     }
//! }
//!
//! #[whichever::dispatch(Shape)]
//! #[whichever::dispatch(Drawable)]
//! pub enum AnyShape {
//!     Circle(Circle),
//!     Square(Square),
//! }
//!
//! fn main() {
//!     let square = AnyShape::from(Square(3));
//!     assert_eq!(square.draw(), "square of area 9");
//! }
//! ```
//!
//! The conversions belong to the enum, not to a trait. They are written
//! once, however many attributes the enum has. A conversion option on any
//! one attribute applies to all of them, so the attributes must agree:
//!
//! - **`from`**: `from = false` on any attribute leaves the `From` impls
//!   out. Two attributes may give `from` the same value. An attribute that
//!   gives it the other value contradicts the first, and you get an error
//!   on the later one.
//! - **`try_from`**: the forms add up, and the widest one on any attribute
//!   applies. So `try_from = false` next to another form cancels nothing.
//! - **`into`**: the targets of all the attributes are collected. A target
//!   listed twice, on two attributes or on one, would be the same `From`
//!   written twice, and you get an error on the second.
//!
//! ## Generic traits
//!
//! A generic trait is forwarded at one instantiation, which you write on the
//! enum:
//!
//! ```
//! pub struct Json;
//! pub struct Text;
//!
//! #[whichever::dispatchable]
//! pub trait Encoder<F> {
//!     fn encode(&self) -> String;
//! }
//!
//! pub struct Point(pub u32, pub u32);
//! pub struct Label(pub &'static str);
//!
//! impl Encoder<Json> for Point {
//!     fn encode(&self) -> String {
//!         format!("[{}, {}]", self.0, self.1)
//!     }
//! }
//! impl Encoder<Text> for Point {
//!     fn encode(&self) -> String {
//!         format!("{} {}", self.0, self.1)
//!     }
//! }
//! impl Encoder<Json> for Label {
//!     fn encode(&self) -> String {
//!         format!("{:?}", self.0)
//!     }
//! }
//!
//! #[whichever::dispatch(Encoder<Json>)]
//! pub enum Value {
//!     Point(Point),
//!     Label(Label),
//! }
//!
//! fn main() {
//!     assert_eq!(Value::from(Point(1, 2)).encode(), "[1, 2]");
//!     assert_eq!(Value::from(Label("hi")).encode(), "\"hi\"");
//! }
//! ```
//!
//! The details:
//!
//! - **The enum's own parameters** work too:
//!   `#[whichever::dispatch(GridTrait<T>)]` on `enum Grid<T>`. The enum
//!   itself may be generic, and its parameters, bounds and `where` clause
//!   carry into the impl.
//! - **A parameter with a default** may be left out:
//!   `#[whichever::dispatch(Enc)]` for `trait Enc<F = Json>`, as
//!   `impl Enc for A` leaves it out.
//! - **Substitution.** Type, lifetime and const parameters are replaced by
//!   their arguments in every signature.
//! - **Projections.** A projection such as `F::Value` becomes
//!   `<Json as Format>::Value` when `F` has exactly one bound. It is written
//!   with the bound's generic arguments and without any constraint on an
//!   associated type, so `F: Format<Text, Value = u64>` gives
//!   `<Json as Format<Text>>::Value`. With several bounds, the trait has to
//!   spell `<F as Format>::Value` itself.
//! - **Payloads that are type parameters.** A payload that is a type
//!   parameter of the enum gives the impl the bound you would write by hand:
//!   `Raw(Raw)` on `enum Lazy<Raw, Parsed>`, or `Box<Raw>` under `deref`,
//!   gives `Raw: Trait`. A struct's field works the same way. A payload
//!   that only mentions a parameter, `Boxed<T>`, gives none: what its own
//!   impl asks of `T` is not written in the enum. An empty partial impl
//!   block writes that header, `impl<T: Shape> Shape for AnyShape<T> {}`.
//! - **Any other header.** A trait generic over its own parameter, for
//!   every `F`, needs a header the attribute cannot write. An empty
//!   [partial impl block](#the-impl-header) gives it one.
//!
//! ## Reaching the target: pointers, fields and methods
//!
//! By default each arm calls the trait method on the payload. Sometimes that
//! is not the value you want. The payload may be a `Box` around the real
//! thing, or a struct whose field implements the trait. A *route* says:
//! start from the payload, take these steps, and call the trait method on
//! what you reach. What you reach is the *target*.
//!
//! The everyday route is `deref`. Mark a variant `#[dispatch(deref)]`, and
//! the trait is called on what the payload dereferences to. So `Box<T>`,
//! `Rc<T>`, `Arc<T>`, `&T` and `&mut T` payloads are forwarded like a plain
//! `T`:
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
//!     assert_eq!(Shape::Shared(Rc::new(Square(4))).area(), 16);
//!     assert_eq!(Shape::Borrowed(&square).area(), 9);
//! }
//! ```
//!
//! ### The steps of a route
//!
//! | Step | Reaches |
//! | --- | --- |
//! | `*` | what the payload dereferences to; `#[dispatch(deref)]` is another way to write this route |
//! | `.field` or `.0` | a field, by name or by position |
//! | `.method()` | what a method returns; the method takes no arguments |
//!
//! You can chain the steps in any order: `via = .account.base` (a field of
//! a field), `via = *.base` (a field of what the payload points at),
//! `via = .inner.base()` (what a field's method returns). `deref` already
//! is the route `*`, so a variant takes either `deref` or `via = ..`, not
//! both.
//!
//! ### Where you write a route
//!
//! - **On a variant or a struct field**, for every trait:
//!   `#[dispatch(via = .base)]`. A second route, `via_mut = route`, serves
//!   the methods that borrow the target mutably; see below.
//! - **On one field of a variant with several**, as the marker:
//!   `Node { id: u32, #[dispatch] data: Data }`. The arm's pattern skips the
//!   other fields with `_` or `..`. The marked field takes `deref`,
//!   `via = route` or `via_mut = route`, and nothing else.
//! - **On the enum's attribute, for one trait**, when the route through one
//!   variant differs from trait to trait: `via(Variant = route)` after the
//!   trait path, as in the example below. For that trait, it wins over both
//!   of the variant's own routes. `via_mut(Variant = route)` given alone
//!   replaces the variant's `via_mut` only.
//! - **On an inherent method, or a marked method of a partial impl block**,
//!   for that method alone, whatever it borrows; see
//!   [inherent methods](#inherent-methods).
//!
//! ```
//! # #[whichever::dispatchable]
//! # pub trait Area { fn area(&self) -> u32; }
//! # #[whichever::dispatchable]
//! # pub trait Named { fn name(&self) -> String; }
//! # pub struct Label(pub &'static str);
//! # pub struct Square(pub u32);
//! # impl Named for Label { fn name(&self) -> String { self.0.to_string() } }
//! # impl Area for Square { fn area(&self) -> u32 { self.0 * self.0 } }
//! # impl Named for Square { fn name(&self) -> String { "square".into() } }
//! #[whichever::dispatch(Area, via(Pair = .1))]
//! #[whichever::dispatch(Named, via(Pair = .0))]
//! pub enum Any {
//!     Pair((Label, Square)),
//!     Plain(Square),
//! }
//!
//! fn main() {
//!     let pair = Any::Pair((Label("tile"), Square(3)));
//!     assert_eq!((pair.area(), pair.name()), (9, "tile".to_string()));
//!     assert_eq!(Any::Plain(Square(2)).name(), "square");
//! }
//! ```
//!
//! ### What the arm looks like
//!
//! The arm stays one line, and it borrows the target as the receiver asks:
//! `&inner.base` for `&self`, `&mut inner.base` for `&mut self`.
//!
//! When the attribute can name the target's type, the call names it. That
//! is the case for a marked field, `<Square as Shape>::area(inner)`, and
//! for a `deref` route,
//! `<<Box<Square> as Deref>::Target as Shape>::area(&**inner)`. The type of
//! a field or of a method's result cannot be named from inside the macro.
//! Then the call is qualified by the trait alone:
//! `<_ as Shape>::area(&inner.base)`. It is still a fully qualified call,
//! so an inherent method of the same name on the target never takes over.
//!
//! On a `deref` variant, the trait is called on
//! `<Payload as Deref>::Target`. The receiver and any `Self` argument are
//! dereferenced to match:
//!
//! - `&**inner` for `&self`;
//! - `&mut **inner` for `&mut self`, so the payload must implement
//!   `DerefMut`;
//! - `*inner` for `self`. A `Box` moves out. A value cannot be moved out of
//!   a reference or a shared pointer, so there you get an error.
//!
//! ### What a route cannot do
//!
//! - **A route that ends in a method call hands over what the method
//!   returns**, so one method serves one way of borrowing. A getter pair
//!   takes two routes: `#[dispatch(via = .source(), via_mut =
//!   .source_mut())]`. A method that borrows the target mutably (`&mut
//!   self`, `self: Pin<&mut Self>` or a `&mut Self` argument) follows
//!   `via_mut`, every other method follows `via`, and each `Self` argument
//!   takes the route of its own borrow. Without `via_mut`, such a method
//!   through a method route is an error that names it. What the route's
//!   method returns is rustc's to judge, at the method's name in the route:
//!   a method returning its target by value under `&self` is "expected
//!   `&_`" there, and is written by hand.
//! - **A method returning `Self` cannot use a routed variant**, `deref`
//!   included, nor a variant with other fields beside the marked one. The
//!   result is a new target, and the rest of the payload is gone, so there
//!   is nothing to wrap it back into. A variant whose one field is named and
//!   marked is the exception: it is rebuilt by its braces.
//! - **No `From` or `TryFrom`** for a variant with other fields beside the
//!   payload, or with its payload in a named field.
//! - **No agreement check** for a target without a nameable type, such as
//!   one reached through a field or a method.
//!
//! ### An open set: `Box<dyn Trait>`
//!
//! A `deref` variant can hold a trait object, `Box<dyn Trait>` or
//! `&dyn Trait`. This is how an enum holds an open set beside its sized
//! payloads: the listed types stay inline, and any other implementor fits
//! in the trait object.
//!
//! A method the trait declares with `where Self: Sized` cannot be called on
//! a trait object, so you get a one-sentence error for it. Leave that method
//! to the trait's default with `#[dispatch(default)]` in a partial impl
//! block, as you would by hand, and the rest is forwarded:
//!
//! ```
//! #[whichever::dispatchable]
//! pub trait Shape {
//!     fn area(&self) -> u32;
//!     fn doubled(&self) -> u32
//!     where
//!         Self: Sized,
//!     {
//!         2 * self.area()
//!     }
//! }
//!
//! pub struct Square(pub u32);
//! pub struct Triangle(pub u32); // Say, from another crate.
//! # impl Shape for Square { fn area(&self) -> u32 { self.0 * self.0 } }
//! # impl Shape for Triangle { fn area(&self) -> u32 { self.0 * self.0 / 2 } }
//!
//! #[whichever::dispatch]
//! pub enum AnyShape {
//!     Square(Square),
//!     #[dispatch(deref)]
//!     Other(Box<dyn Shape>),
//! }
//!
//! #[whichever::dispatch]
//! impl Shape for AnyShape {
//!     #[dispatch(default)]
//!     fn doubled(&self) -> u32
//!     where
//!         Self: Sized;
//! }
//!
//! fn main() {
//!     let other = AnyShape::Other(Box::new(Triangle(4)));
//!     assert_eq!((other.area(), other.doubled()), (8, 16));
//!     assert_eq!(AnyShape::Square(Square(2)).doubled(), 8);
//! }
//! ```
//!
//! Some pointers implement the trait themselves: `Box<dyn Iterator>` is an
//! `Iterator`. Hold such a pointer without `deref`. Then every method,
//! `Self: Sized` or not, is forwarded to the pointer itself, and you need no
//! partial impl block.
//!
//! ## Inherent methods
//!
//! An enum often needs methods of its own, outside any trait, that only some
//! variants can provide. Put `#[dispatch]` on an inherent `impl` block of a
//! dispatch enum, an *inherent block*, and mark the methods to forward. A
//! marked method is forwarded to the method of the same name on the listed
//! variants' payloads. Its body is the *fallback*: what every other variant
//! returns.
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
//!     let circle: AnyShape = Circle(2).into();
//!     let square: AnyShape = Square(2).into();
//!     assert_eq!(circle.radius(), Some(2));
//!     assert_eq!(square.radius(), None);
//!     assert_eq!(square.describe(), "square 2");
//! }
//! ```
//!
//! What to notice:
//!
//! - `radius` calls `Circle::radius` and wraps the result in `Some`. A
//!   `Square` returns the body, `None`.
//! - `*` means every variant, so `describe` has no body: the signature ends
//!   with `;`. A method that lists every variant one by one has no body
//!   either, since no variant is left for a body to serve.
//! - Methods without `#[dispatch(..)]` are left alone.
//!
//! The options follow the variants in the attribute, and change what each
//! forwarding arm becomes:
//!
//! | Option | The arm becomes |
//! | --- | --- |
//! | (none) | `inner.method(args)` |
//! | `with = path` | `path(inner.method(args))` |
//! | `call = name` | `inner.name(args)` |
//! | `field = name` | the field `inner.name`, borrowed or read |
//! | `value` | the target itself, borrowed or read |
//! | `via = route` | the call or place, on the target the route reaches |
//!
//! ### Forward to every variant but a few
//!
//! Under `*`, a method may still have a body. The body is then one
//! `match self` that holds the arms the enum handles itself, and every
//! variant those arms do not name is forwarded. Use it for a unit variant
//! beside the payloads:
//!
//! ```
//! # pub struct Circle(pub u32);
//! # pub struct Square(pub u32);
//! # impl Circle { pub fn describe(&self) -> String { format!("circle {}", self.0) } }
//! # impl Square { pub fn describe(&self) -> String { format!("square {}", self.0) } }
//! #[whichever::dispatch]
//! pub enum AnyShape {
//!     Circle(Circle),
//!     Square(Square),
//!     Empty,
//! }
//!
//! #[whichever::dispatch]
//! impl AnyShape {
//!     #[dispatch(*)]
//!     pub fn describe(&self) -> String {
//!         match self {
//!             Self::Empty => String::from("nothing"),
//!         }
//!     }
//! }
//!
//! fn main() {
//!     assert_eq!(AnyShape::Empty.describe(), "nothing");
//!     assert_eq!(AnyShape::Square(Square(2)).describe(), "square 2");
//! }
//! ```
//!
//! A bare `#[dispatch]` on the enum, with no trait, registers it for
//! inherent blocks without forwarding any trait.
//!
//! The generated `match` holds the forwarding arms first, in the enum's
//! order, then your arms as you wrote them. It has no wildcard arm. So a
//! variant you add to the enum later is forwarded, and a variant without a
//! payload and without an arm of yours gives an error on the method. Your
//! arms:
//!
//! - name their variants by path, `Self::Empty`, possibly through a guard,
//!   an or-pattern, a reference or a binding `name @ ..`. An arm that names
//!   no variant, `_` or a binding, is an error;
//! - are written in the method's declared return type, with no `with`
//!   around them;
//! - may read the fields their pattern binds;
//! - under `#[cfg]`, handle their variant where the `cfg` is on. Where it
//!   is off, the variant is forwarded.
//!
//! The body is the `match self` and nothing else.
//!
//! #### Code around the `match`
//!
//! The method you would write by hand often has a line or two before its
//! `match`, such as a check. Under `*` that body gives an error, because
//! the attribute writes the whole body: one `match` with the forwarding
//! arms and yours, with no place for other code. Split the method in two
//! instead: a plain method, without a mark, that runs the code and then
//! calls a private method marked `#[dispatch(*, call = name)]`, where
//! `name` is the payloads' method, whose body is the `match self` with
//! your arms:
//!
//! ```
//! # pub struct Circle(pub u32);
//! # pub struct Square(pub u32);
//! # impl Circle { pub fn describe(&self) -> String { format!("circle {}", self.0) } }
//! # impl Square { pub fn describe(&self) -> String { format!("square {}", self.0) } }
//! # #[whichever::dispatch]
//! # pub enum AnyShape {
//! #     Circle(Circle),
//! #     Square(Square),
//! #     Empty,
//! # }
//! #[whichever::dispatch]
//! impl AnyShape {
//!     pub fn describe(&self, verbose: bool) -> String {
//!         if !verbose {
//!             return String::from("a shape");
//!         }
//!         self.describe_variant()
//!     }
//!
//!     #[dispatch(*, call = describe)]
//!     fn describe_variant(&self) -> String {
//!         match self {
//!             Self::Empty => String::from("nothing"),
//!         }
//!     }
//! }
//!
//! fn main() {
//!     assert_eq!(AnyShape::Square(Square(2)).describe(false), "a shape");
//!     assert_eq!(AnyShape::Square(Square(2)).describe(true), "square 2");
//!     assert_eq!(AnyShape::Empty.describe(true), "nothing");
//! }
//! ```
//!
//! The error on such a body names this way around, with the payloads'
//! method in it. A body with no `match self` at all gets the other error,
//! "its body would never run". A trait method works the same way: write
//! it in full in a [partial impl
//! block](#partial-impl-blocks-write-some-methods-yourself), and have it
//! call the inherent method that holds the `match`.
//!
//! ### How the call is written
//!
//! Each arm calls the payload's method with method-call syntax,
//! `inner.describe()`. So autoref, autoderef, and the rule that an inherent
//! method wins over a trait method of the same name all work as in
//! hand-written code.
//!
//! On a `deref` variant, the call goes to what the payload points at:
//! `(**inner).describe()`. A method the pointer itself has under the same
//! name never takes over. `Box` has an `as_ref`, for example, but a
//! forwarded `as_ref` calls the one on the boxed value.
//!
//! ### The options in detail
//!
//! - **`with = path`** passes each forwarded result through `path` on its
//!   way out, as `Some` does above. `path` may be a generic function. It is
//!   called separately in each arm, so payloads that return different types
//!   can meet in one declared return type. `with = Into::into` rebuilds the
//!   enum through its `From` impls; use it to return the enum at another
//!   lifetime.
//! - **`call = name`** forwards to a payload method of another name:
//!   `#[dispatch(*, call = describe)] pub fn label(&self) -> String;`.
//! - **`field = name`** makes the arm read that field of the target. The
//!   field is borrowed when the method returns a reference, and read by
//!   value otherwise.
//! - **`value`** makes the arm the target itself, for `with` to wrap or for
//!   the return type to widen. Like a field, it is borrowed where the method
//!   returns a reference and read where it returns a value. So
//!   `#[dispatch(Controller, value, with = Some)]` on
//!   `fn as_owned(&self) -> Option<&dyn Owned>` is `Some(inner)`, and
//!   `#[dispatch(*, value)]` on `fn handle(&self) -> Handle` is `*inner`,
//!   on a payload or a marked field alike. `via = .name, value` is
//!   `field = name` spelled out.
//! - **`via = route`** reaches the target for this method alone. The route
//!   starts at the payload, and wins over the variant's own route if the
//!   variant has one.
//!
//! How `field` and `value` read the return type:
//!
//! - Under `with = Some` or `with = Ok`, the borrow is read inside the
//!   `Option` or the `Result`: `-> Option<&T>` with `field = name` gives
//!   `Some(&inner.name)`.
//! - Any other `with` is handed the target as it stands. Under `&self`,
//!   that is the reference the pattern binds. For a place reached through
//!   `via` or `field`, it is the place by value.
//! - A method returning a raw pointer is also handed the target as it
//!   stands, and so is a method returning an `impl Trait` on a dispatch
//!   struct.
//! - The return type is read as written, so a type alias for a reference
//!   reads as a value.
//!
//! ### The fine print
//!
//! - **Methods that return `Self`.** A method returning `Self`,
//!   `Option<Self>` or `Result<Self, E>` wraps the payload's result back
//!   into the variant, as trait dispatch does. With `with`, the payload
//!   returns the plain value and `with` lifts the wrapped variant. So
//!   `#[dispatch(Circle, with = Some)]` on
//!   `fn grown(&self) -> Option<Self>` calls a `Circle::grown` that returns
//!   a `Circle`.
//! - **`async` methods**, and methods that return `impl Future`, are
//!   awaited in each arm.
//! - **`Self` arguments.** A `Self` argument is the enum itself. It is
//!   handed to the payload's method as it is.
//! - **`self` by value on a `deref` variant.** A value cannot be moved out
//!   of a `deref` variant that holds anything but a `Box`. So there, `field`
//!   or `value` under `self` or `self: Box<Self>` gives an error, unless the
//!   return type borrows the place. A method forwarded as a call, with no
//!   `field` or `value`, is the arm you would write by hand under `self`,
//!   `(*inner).name()`. The payload's method decides what moves: one
//!   taking `&self` borrows what the variant points at.
//! - **Variants under `#[cfg]`** may be listed. In a build where the
//!   variant is compiled out, the method uses its body.
//! - **Methods under `#[cfg]`.** A method that cannot be forwarded gives its
//!   error under the method's own `#[cfg]`, so a build that compiles the
//!   method out gives no error.
//! - **Where the block goes.** The enum keeps its definition in a
//!   crate-local macro for its blocks to read. So the `impl` block has to be
//!   in the enum's crate, which inherent impls are anyway.
//!
//! ### On a struct
//!
//! An inherent block on a [dispatch struct](#forwarding-from-a-struct-field)
//! forwards the same way:
//!
//! - `*` is the field marked with a bare `#[dispatch]`, or the only field.
//! - A field's name is that field, marked or not.
//! - A method lists one field, since every value of the struct holds them
//!   all. Listing two gives an error.
//! - A `Self` argument is the other value's field, as in trait dispatch.
//! - A return-position `impl Trait` is forwarded as it is.
//!
//! ## Partial impl blocks: write some methods yourself
//!
//! Forwarding does not fit every trait method. On an enum, a method without
//! a receiver, such as a constructor, has no variant to forward to. And sometimes the
//! enum should return something different from every payload.
//!
//! For these, write a *partial impl block*: a trait impl block with
//! `#[dispatch]` on it. You write the items the enum handles itself, and the
//! rest of the trait is forwarded around them:
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
//! `from_any` is yours. `is_attached` and `describe` are forwarded, as if
//! the enum had `#[dispatch(Handler)]`.
//!
//! The basics:
//!
//! - **The enum does not list the trait in its own attribute.** The block
//!   is the impl. If you list the trait in both places, you get an error on
//!   the block.
//! - **The trait's options go on the block's attribute**: overrides such as
//!   `Name = Type`, and `mismatch`. On the enum's own attribute, which lists
//!   no trait, they are an error, since no impl would read them there.
//! - **An associated type written in the block is an override too**, as if
//!   you had written `Name = Type` on the attribute.
//! - **An enum's generic associated type is declared here**, and so is a
//!   struct's whose field is reached through a route. No first variant can
//!   supply one, so you declare it in the block,
//!   `type Item<'a> = AnyItem<'a> where Self: 'a;`, and each payload's own
//!   type converts into it through `Into`, as for any override.
//! - **The block has to be in the enum's crate**, as an inherent block
//!   does.
//!
//! ### Write a method, or mark it
//!
//! A method you write in the block replaces the forwarding for every
//! variant. This includes provided methods, so it is also the way to leave
//! out a payload's version of a method on purpose. A method written under
//! `#[cfg]` replaces the forwarding where the `cfg` is on, and is forwarded
//! where it is off, as you would write the two versions by hand.
//!
//! A *mark* is `#[dispatch(..)]` on a method of a block, or `#[dispatch]`
//! on a field. Instead of writing a method, you may give it a mark that
//! lists the variants it is forwarded to, as an inherent method does:
//! `#[dispatch(Wep, Wpa)] fn fmt(..) { .. }`. Its body is then the arm for
//! every other variant. That includes a unit variant, and the case where
//! `self` and a `Self` argument hold two different variants.
//!
//! The other marks:
//!
//! - **`#[dispatch(*)]` on a method without a body** is the plain
//!   forwarding's arms, written out.
//! - **`#[dispatch(*)]` on a method with a body** holds the arms the enum
//!   handles itself, as in an inherent block:
//!   `#[dispatch(*)] fn encode(..) { match self { Self::Unknown(kind, data) => .. } }`.
//!   Every other variant is forwarded. The body is the `match` and
//!   nothing else; for code around it, see "Calling the payloads from a
//!   method you wrote" below. A trait method with a `Self` argument,
//!   matched beside `self`, takes no such arms; list the forwarded
//!   variants instead and give the method a body for the rest.
//! - **`#[dispatch(default)]` on a method without a body** leaves the method
//!   to the trait's default. Over a
//!   [restatement](#traits-from-std-and-other-crates), that is the real
//!   trait's default.
//!
//! Marks and `#[cfg]`:
//!
//! - A mark under `#[cfg]`, `#[dispatch(default)]` included, applies where
//!   the `cfg` is on, as a written method does. Where it is off, a mark of
//!   the same name without `#[cfg]` applies, if there is one. If there is
//!   none, the method is forwarded to every variant.
//! - A variant that no method can be forwarded to, such as a unit variant,
//!   gives an error in the builds in which some method is forwarded to
//!   every variant, and in no other build.
//!
//! ### Attributes on the methods
//!
//! A method you write or mark in the block carries what you write on it,
//! and nothing of the trait method's own attributes. A method you write in
//! full is emitted as you wrote it, with nothing added. A method you mark
//! to be forwarded gets `#[inline]` unless you wrote an `#[inline(..)]`
//! yourself, since the attribute writes its `match`. Only the methods
//! the block leaves to plain forwarding carry the trait method's
//! `#[inline(..)]` and `#[allow]`.
//!
//! ### The impl header
//!
//! The block's header, generics and bounds included, becomes the impl's
//! header. It may refer to the enum's parameters however it likes:
//! `impl<T, Ty> Get<T> for ImmOrNode<Ty>`.
//!
//! So an empty block is how the impl gets a header that the enum's
//! attribute cannot write:
//!
//! - generics the enum does not have, as for a trait generic over its own
//!   parameter, `impl<F: Format> Encoder<F> for AnyEncoder {}`, or a
//!   lifetime, `impl<'r> Renderable<'r> for Node {}`;
//! - one instantiation of a generic enum;
//! - a bound on a type parameter beyond the `Param: Trait` the impl gets on
//!   its own. That bound comes only from a payload that is the parameter
//!   itself; a payload that mentions it, `Boxed<T>`, needs the block's
//!   `impl<T: Shape> Shape for AnyShape<T> {}`.
//!
//! ### Names, attributes and what the block cannot hold
//!
//! - **Names in scope.** The forwarding arms name the payload types as the
//!   enum spells them, so a block in another module needs them in scope
//!   there. It also works the other way round. When the trait's signature
//!   types are not in scope where the enum is, an empty block in a module
//!   where they are, such as one holding `use trait_crate::*;`, is the
//!   place to write the impl.
//! - **Attributes on the block** are carried onto the impl. So
//!   `#[typetag::serde(name = "..")]` goes on the block, with
//!   `#[whichever::dispatchable]` above `#[typetag::serde]` on the trait.
//!   Then typetag's own methods belong to the block and are never
//!   forwarded.
//! - **A macro invocation in the block gives an error**, because the items
//!   it writes cannot be told from the ones to forward. Write those items
//!   out in the block. A macro invocation under `#[cfg]` gives an error only
//!   where the `cfg` is on; where it is off, the block is read without it.
//! - **The same goes for every other item the block cannot read**: a method
//!   without a body or a mark, a mark that does not parse, a mark with an
//!   inherent method's option, a mark beside a second mark, and
//!   `#[dispatch(default)]` on a method with a body.
//! - **Calling the payloads from a method you wrote.** A method you write
//!   in the block may still need the payloads' result. Have it call an
//!   inherent `#[dispatch(*, call = name)]` method beside it. That method
//!   may hold arms of the enum's own, which is the way to have code around
//!   a `match` with your arms; see [Code around the
//!   `match`](#code-around-the-match).
//!
//! A partial impl block works on a
//! [dispatch struct](#forwarding-from-a-struct-field) as it does on an enum.
//!
//! ## Traits from `std` and other crates
//!
//! A trait from `std`, or from any crate you do not own, never had
//! `#[dispatchable]` on it, so `#[dispatch]` has nothing to read its
//! signatures from. For the common ones, the [`foreign`] module has
//! ready-made restatements. Here an error enum gets `Display` without a
//! hand-written `match`:
//!
//! ```
//! use std::num::ParseIntError;
//!
//! pub struct TooBig(pub u32);
//! # impl std::fmt::Display for TooBig {
//! #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//! #         write!(f, "{} is too big", self.0)
//! #     }
//! # }
//!
//! #[whichever::dispatch(whichever::foreign::Display)]
//! pub enum ShapeError {
//!     Parse(ParseIntError),
//!     TooBig(TooBig),
//!     #[dispatch(from = false)]
//!     Unknown(String),
//! }
//!
//! fn main() {
//!     let parse: ShapeError = "x".parse::<u32>().unwrap_err().into();
//!     let too_big = ShapeError::from(TooBig(1000));
//!     let unknown = ShapeError::Unknown("hexagon".to_string());
//!     assert_eq!(parse.to_string(), "invalid digit found in string");
//!     assert_eq!(too_big.to_string(), "1000 is too big");
//!     assert_eq!(unknown.to_string(), "hexagon");
//! }
//! ```
//!
//! Each variant prints with its payload's own `Display`. The `String`
//! variant stays out of the `From` impls with `#[dispatch(from = false)]`,
//! so a plain string does not turn into a `ShapeError` by accident.
//!
//! [`foreign`] holds `Display`, `Debug`, `Error`, `PartialEq`, `Iterator`,
//! `DoubleEndedIterator`, `ExactSizeIterator`, `Read`, `Write`, `BufRead`
//! and `Seek`. Each one forwards the provided methods a `match` can
//! forward, as the module lists them. So `foreign::Iterator` reaches a
//! payload's own `any`, `find`, `max` or `collect`.
//!
//! ### Restate any other trait
//!
//! For any other trait, restate it once with `remote` set to the path of the
//! real trait, and dispatch the restatement. Nothing is emitted for the
//! *restatement* but the carrier macro. Dispatching it implements the real
//! trait, forwarding exactly the methods restated:
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
//! The rules:
//!
//! - **Write the signature types in full**, as [`foreign`] does, or import
//!   them where the enum is. Nothing but the macro reads the restatement, so
//!   its own imports count as unused.
//! - **Bounds on a restated associated type may be left out.**
//! - **An `#[async_trait]` trait**: put `#[async_trait]` above
//!   `#[dispatchable(remote = ..)]` on the restatement, as on the trait
//!   itself.
//! - **List the provided methods a payload overrides.** A provided method
//!   the restatement leaves out keeps the trait's default on the enum,
//!   whatever the payload overrides.
//!
//! tokio's `AsyncWrite` is the common case for the last rule. Its streams
//! take every buffer in one `poll_write_vectored` call. A restatement of
//! `tokio::io::AsyncWrite` lists `poll_write_vectored` and
//! `is_write_vectored` beside the three required methods. Left out, the
//! enum reports `false` from `is_write_vectored`, whatever its payload
//! returns, and a caller such as hyper copies every buffer into one before
//! writing it.
//!
//! ## Enums from other crates
//!
//! The same trick works the other way round. You can forward a trait of
//! your crate over an enum another crate defines, as a hand-written
//! `impl LocalTrait for other::Either<L, R>` could. Restate the enum once,
//! with `remote` set to the real one, and dispatch the restatement:
//!
//! ```rust,ignore
//! #[whichever::dispatch(remote = either::Either)]
//! #[whichever::dispatch(Describe)]
//! enum Either<L, R> {
//!     Left(L),
//!     Right(R),
//! }
//! ```
//!
//! - Nothing is emitted for the restatement but its carrier macro and the
//!   impls, and the impls are written for the real path.
//! - A restated enum gets no conversions and no inherent impl. The orphan
//!   rules and the language refuse both for a type from another crate.
//! - For a partial impl block, import the real enum under the restatement's
//!   name and write the block on it.
//! - A struct from another crate is restated the same way, with `remote` on
//!   its attribute. The restatement lists only the fields its impls forward
//!   to: the real struct may have more, and the impl's pattern ends in `..`.
//!   It gets no inherent impl either, and no conversions, as no struct does.
//!
//! ## Forwarding from a struct field
//!
//! A dispatch enum is often private, behind a public struct that forwards
//! the same trait. Put `#[dispatch(Trait)]` on the struct. It forwards to
//! the struct's one field, or to the field marked `#[dispatch]`, and it
//! forwards every method, provided ones included. So a payload's override
//! reaches the public type:
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
//! The field does not have to be a dispatch enum. Any implementor works, so
//! the same attribute also replaces a field-delegation derive.
//!
//! **A struct with several fields marks one.** Without a mark the attribute
//! cannot tell which field is meant, and you get an error that says to mark
//! it with `#[dispatch]` or to name the trait on it, `#[dispatch(Trait)]`.
//! Where a field is a `Box`, an `Rc`, an `Arc` or a reference to the target,
//! the error also names `#[dispatch(deref)]`, which forwards to what the
//! pointer points at; a bare mark forwards to the pointer itself, which is
//! right only where the pointer implements the trait. A pointer to a string
//! or a slice, such as `&'static str`, is not taken for one. The impl is
//! still written, so this error is the only one.
//!
//! **Several traits, several fields.** A field may list the traits it is the
//! target of: `#[dispatch(Rom)] rom: Cartridge` beside
//! `#[dispatch(Interrupt)] signals: Signals`. The field marked with a bare
//! `#[dispatch]` takes every trait no other field lists.
//!
//! **Where a trait's impl lives.** A trait's impl is written by the struct's
//! attribute, or by a partial impl block when it needs a header of its own,
//! such as
//! `impl<S: Sorts, C: Catalog<Extractor = S::Extractor>, N> Catalog for Env<S, C, N> {}`.
//! Either way, the field that lists the trait is its target.
//!
//! **How a field spells a trait.** A field writes a trait as the impl's
//! home spells it, arguments included: `#[dispatch(Encoder<F>)]` for
//! `impl<F, ..> Encoder<F> for ..`. So two instantiations of one generic
//! trait, or two traits of one name from two modules, can go to two fields.
//! One name spelled two ways, `Catalog` on the field and `traits::Catalog`
//! at the home, gives an error. The impl is still written for that field,
//! so this error is the only one.
//!
//! **Wrap some methods, forward the rest.** A struct that wraps a few
//! methods, with a retry or a check before the call, writes them in a
//! partial impl block, and every other method is forwarded as it is. A
//! method added to the trait later is forwarded too, without the
//! wrapping, and nothing reports it. `#[deny(clippy::missing_trait_methods)]`
//! on the block reports nothing: the lint does not look at an impl a macro
//! writes, and the generated impl holds every method anyway. When every
//! new method has to be looked at, write the impl by hand under that lint.
//!
//! What is different from an enum:
//!
//! - **`#[dispatch(deref)]` on the field** reaches through a `Box` or
//!   another pointer.
//! - **A method returning `Self`** is rebuilt only around a struct with no
//!   other field. Otherwise it goes in a partial impl block, which works on
//!   a struct as on an enum.
//! - **`Option<Self>` and `Result<Self, E>`** are mapped with a path, which
//!   a struct with a named field does not have. So there they are an error,
//!   and the tuple form, `struct Named(Square);`, avoids it. A plain
//!   `-> Self` is rebuilt by the braces, on such a struct and on a variant
//!   whose one field is named and marked.
//! - **A return-position `impl Trait` is forwarded as it is.** A struct's
//!   `match` has one arm, so there is only one hidden type. An enum cannot
//!   do this.
//! - **A struct gets no `From` impls.**
//! - **`Self` arguments need no `mismatch`.** Two values of the struct always
//!   hold the same field, and the argument is unwrapped to it.
//! - **What needs no value comes from the field's type**, where the struct
//!   names it: the one field or the marked one, directly or through
//!   `deref`. A trait method without a receiver is
//!   `<Square as Shape>::kind()`, the provided ones included; a generic
//!   associated type is `<Square as Shape>::Side<'a>`; and an inherent
//!   method without a receiver, marked in a block, is a path call such as
//!   `#[dispatch(*, call = from)] pub fn from_u64(v: u64) -> Self;`. A
//!   function with a `Self` argument, a field holding a trait object and a
//!   field reached through a route stay as on an enum.
//!
//! # Reference
//!
//! ## Options
//!
//! Options on the enum go inside `#[dispatch(..)]`, after the trait path, as
//! in `#[whichever::dispatch(Storage, Error = StorageError)]`. A few go on a
//! variant, on a trait method or on a marked field instead.
//!
//! | Option | Where you write it | What it does |
//! | --- | --- | --- |
//! | [`Name = Type`](#associated-types-and-consts) | on the enum, after the trait path | Fixes an associated type; results are converted into it. |
//! | [`Name = value`](#associated-types-and-consts) | on the enum, after the trait path | Fixes an associated const. |
//! | [`mismatch(method = expr)`](#self-arguments-and-mismatch) | on the enum, after the trait path | What `method` returns when `self` and a `Self` argument hold different variants. |
//! | [`mismatch = expr`](#self-arguments-and-mismatch) | on a trait method | The same, decided once by the trait's author. |
//! | [`from = false`](#conversions) | on the enum, after the trait path | No `From<Payload>` impls. |
//! | [`try_from = true`, `ref`, `ref_mut`](#conversions) | on the enum, after the trait path | Adds `TryFrom<Enum> for Payload`, and the by-reference forms. |
//! | [`into = Target`](#conversions) | on the enum, after the trait path | Adds `From<Enum> for Target`. |
//! | [`via(Variant = route)`](#where-you-write-a-route) | on the enum, after the trait path | A route through `Variant` for this trait only. |
//! | [`via_mut(Variant = route)`](#where-you-write-a-route) | on the enum, after the trait path | The same, for the methods that borrow the target mutably. |
//! | [`from = false`](#conversions) | on a variant | Leaves the variant out of `From` and `TryFrom`. |
//! | [`try_from = true`](#conversions) | on a variant | Adds `TryFrom` for this variant's payload alone. |
//! | [`deref`](#reaching-the-target-pointers-fields-and-methods) | on a variant, or on the marked field | Calls the trait on what the payload dereferences to. |
//! | [`via = route`](#reaching-the-target-pointers-fields-and-methods) | on a variant, or on the marked field | Calls the trait on a field of the payload, or on what a method returns. |
//! | [`via_mut = route`](#reaching-the-target-pointers-fields-and-methods) | on a variant, or on the marked field | The route the methods that borrow the target mutably take instead. |
//! | [`#[dispatch]`](#where-you-write-a-route) | on one field of a variant | Picks the field to forward to, in a variant with several fields or a named one. |
//! | [`#[dispatch(Trait)]`](#forwarding-from-a-struct-field) | on a struct's field | Makes the field the target of that trait. |
//! | [`#[dispatch(remote = path)]`](#enums-from-other-crates) | on an enum or a struct, as an attribute of its own | Restates an enum or a struct another crate defines. |
//!
//! Four options go on a variant: `from = false`, `try_from = true`, `deref`
//! and `via = route`.
//!
//! The marks of an [inherent block](#inherent-methods) (`*`, variant names,
//! `with`, `call`, `field`, `value`, `via`) and of a
//! [partial impl block](#partial-impl-blocks-write-some-methods-yourself)
//! (`default`) are described with them.
//!
//! ## What each trait item becomes
//!
//! | Trait item | In the enum's impl |
//! | --- | --- |
//! | `type X;` | `type X = <FirstVariant as Trait>::X;`, or the `X = Type` option. Every other variant is checked to agree, and a disagreement is an error on that variant. |
//! | `const N: T;` | `const N: T = <FirstVariant as Trait>::N;`, or the `N = value` option. Some consts are checked to agree on every variant. |
//! | method with a receiver | A `match` over the variants calling `<Payload as Trait>::method(payload, args..)`. |
//! | method with a default body | The default is ignored: the method is forwarded like any other. |
//! | generic method | Forwarded with its type and const parameters spelled out when no argument fixes them. |
//! | `self: Box<Self>` | Works for `deref` variants that hold a `Box`. |
//! | `self: Pin<&Self>`, `self: Pin<&mut Self>` | Unpinned, matched and pinned again. Every payload must be `Unpin`. |
//! | `async fn` | Each arm is awaited. |
//! | `unsafe fn` | Each call is wrapped in `unsafe { }`. |
//! | `unsafe trait` | An `unsafe impl`. |
//! | `-> impl Future<Output = T>` | One `async move` block around the `match`, each arm awaited. |
//! | `-> Pin<Box<dyn Future<Output = T> + ..>>`, as `#[async_trait]` writes it | Forwarded as it is. |
//! | any other `-> impl Trait` | An error on an enum. Forwarded as it is on a struct. |
//! | argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`. Different variants return the `mismatch` expression. |
//! | return `Self` | Wrapped back into the same variant. |
//! | return `Self::X` with `X` overridden | Converted with `Into`. |
//! | method without a receiver | On an enum, left to the trait's default body, or written by you in a partial impl block; an error if it has neither. On a struct, `<Target as Trait>::method(args..)`. |
//! | `#[cfg]` on a method or an associated item | Decided in the trait's crate. |
//! | `#[cfg]` on a variant | Carried onto the variant's arm, and decided where the enum is. |
//! | `#[deprecated]` on a variant | Arms and conversions written under `#[allow(deprecated)]`. |
//! | `#[deprecated]` on a method or an associated item | Forwarded under `#[allow(deprecated)]`. |
//! | `#[inline]` | Added to every generated method. |
//! | `#[allow(..)]` on a method or an associated item | Carried onto the generated item. |
//!
//! The notes for the rows that need more than a line:
//!
//! - **Boxed receivers.** For `self: Box<Self>`, the enum is moved out of
//!   its box, and the method is called with the variant's own
//!   `Box<Payload>`. A plain variant is an error.
//! - **Futures.** For `-> impl Future<Output = T>`, the whole `match` goes
//!   into one `async move` block, and every arm is awaited inside it.
//!   `Send` and any other bound on the future are checked against that
//!   block. On a struct, the one arm returns the field's future as it is,
//!   unless its output is rewrapped or converted.
//! - **Boxed futures.** `#[async_trait]`'s
//!   `Pin<Box<dyn Future<Output = T> + ..>>` is one type for every arm, so
//!   it is forwarded as it is. When `T` is rewrapped or converted, every arm
//!   is awaited in one `async move` block, which is boxed and pinned again
//!   with the `Box` the signature names, as `#[async_trait]` itself does.
//! - **Other `impl Trait`.** An error on an enum: a `match` returns one
//!   type, and the attribute cannot tell that the variants' opaque types are
//!   one, even where every variant reaches a target of one type. A struct's
//!   one arm returns one.
//! - **Overridden types.** A return of `Self::X` is converted with `Into`;
//!   inside `Option` or `Result` with `map(Into::into)`, and
//!   `Result<_, Self::X>` with `map_err(Into::into)`. See
//!   [associated types and consts](#associated-types-and-consts).
//! - **`Self` in arguments and returns.** See
//!   [`Self` arguments](#self-arguments-and-mismatch) and
//!   [`Self` in return types](#self-in-return-types).
//! - **`#[cfg]` on trait items.** The carrier macro is written once per
//!   combination of the trait's distinct `cfg` predicates, each copy under
//!   its own `cfg`. So the generated item exists exactly when the trait's
//!   does, in a build of the trait's crate.
//! - **`#[cfg]` on variants.** An error about a variant, from a method that
//!   cannot be forwarded to it, is that arm's result, so it sits under the
//!   variant's `cfg` too. A variant that is compiled out is never an error,
//!   and each variant the method cannot reach gets its own error, naming
//!   it. Associated types and consts come from the first variant that is
//!   always compiled in and whose target has a type the attribute can
//!   name: a variant under `#[cfg]` is passed over, and so is one reached
//!   through a field or a method. When every variant is under `#[cfg]`, a
//!   build may leave none.
//!   For that case, a method whose receiver is a reference matches
//!   `match *self` with `ref` bindings, so the empty build still compiles,
//!   as a hand-written one does.
//! - **`#[deprecated]`.** On a variant, the deprecation still reaches the
//!   code that names the variant. On a trait method or associated item, the
//!   forwarding is written under `#[allow(deprecated)]`, as you would write
//!   it by hand, and your callers still see the deprecation through the
//!   trait's item.
//! - **`#[inline]`.** A trait method's own `#[inline(..)]` is carried
//!   instead. A method a partial impl block writes or marks carries what
//!   you wrote on it in the block.
//! - **`#[allow(..)]`.** The generated item repeats the signature the lint
//!   was allowed on, so it needs the same `allow`. A `warn`, `deny` or
//!   `expect` stays with the trait's crate, and so does an `#[allow]` that
//!   another macro added.
//!
//! ## Across crates
//!
//! `#[dispatchable]` emits the trait unchanged, plus its *carrier macro*: a
//! `#[doc(hidden)]` `macro_rules!` macro that holds the trait's signatures.
//! It is re-exported under the trait's own name. Macros live in their own
//! namespace, so `use shapes::Shape;` imports both.
//!
//! - For a `pub` trait, the macro is exported at the crate root, which is
//!   the only way another crate can reach it. A trait of any other
//!   visibility keeps its macro in its module.
//! - `#[dispatch(Shape)]` invokes that macro, which hands the trait and the
//!   enum to a proc macro that writes the impl.
//! - Nothing is cached between expansions. So the two attributes work
//!   across crates, in any order, and under rust-analyzer.
//!
//! **Versions.** Both crates depend on `whichever`, and before 1.0 on the
//! same minor version. The carrier macro is read by the enum crate's
//! `whichever`. A newer `whichever` reads every older carrier macro; an
//! older one gives a one-sentence error for a newer carrier macro. A
//! renamed dependency is fine.
//!
//! **The types in the signatures.** The impl is written into the enum's
//! module, so the types the trait's signatures name must be in scope there,
//! exactly as for a hand-written impl.
//!
//! - They must also be nameable from there. A private alias or type in a
//!   signature limits the trait to its own crate.
//! - Paths that start with `crate::` are rewritten to the trait's crate.
//!   `super::` and `self::` paths cannot be, so they must not appear in a
//!   dispatchable trait's signatures.
//! - The attribute does not import the types itself, because a glob import
//!   written around the impl would shadow the enum module's own names.
//! - A glob import of the trait's crate in the enum's module,
//!   `use trait_crate::*;`, brings them in, and the module's own names still
//!   win over it.
//! - To keep the glob out of the enum's file, put it in a module that writes
//!   one empty partial impl block per trait,
//!   `#[whichever::dispatch] impl Client for ClientWrapper {}`, with the
//!   payload types imported there by name.
//!
//! ## Requirements and limits
//!
//! A *limit* is something the attributes refuse to forward: a kind of
//! method signature, variant or option. You get a one-sentence error on the
//! item at fault, saying what to do instead where there is something. The
//! rest of the enum still compiles, so you see the error for the thing to
//! fix, and no wall of follow-on errors.
//!
//! ### The enum and its variants
//!
//! - **Every variant a trait is forwarded to holds exactly one unnamed
//!   field, `Name(Payload)`, or marks the field to forward to with
//!   `#[dispatch]`.** The arm needs one value to call the method on. The
//!   error follows the variant's own `#[cfg]`, so a variant that is compiled
//!   out is no error. An enum that only forwards inherent methods may hold
//!   variants of any other form; they take the method's body.
//! - **Every target implements the trait.** The arm calls the trait method
//!   on it. Implement the trait for the payload, or reach a target that
//!   implements it with a route.
//! - **An error about the way a method reaches a variant follows that
//!   variant's `#[cfg]` as well.** Each variant a method cannot reach gets
//!   an error of its own, naming it.
//! - **A payload type used by two variants gets no `From` impl.** A `From`
//!   could not know which variant to build. Build the variant by name.
//! - **The marked field of a variant takes `deref` and `via = route`
//!   only.** `from` and `try_from` go on the variant. A trait listed on the
//!   field is an error, since the variant is the target of every trait its
//!   enum dispatches.
//!
//! ### Receivers
//!
//! - **A receiver is `self`, `&self`, `&mut self`, `self: Box<Self>` on
//!   `deref` variants holding a `Box`, or `self: Pin<&Self>` or
//!   `self: Pin<&mut Self>` when every payload is `Unpin`.** `Rc<Self>`,
//!   `Arc<Self>` and `Pin<Box<Self>>` would need a refcount or an
//!   allocation, so they are not supported.
//! - **On an enum, a method without a receiver needs a default body or an
//!   implementation of your own.** It has no value to match on. A struct
//!   forwards it to its field's type, except where the method takes a
//!   `Self` argument, the field holds a trait object or the field is reached
//!   through a route. Give it a
//!   default body, which the enum uses, or write it in a
//!   [partial impl block](#partial-impl-blocks-write-some-methods-yourself).
//!
//! ### `Self` in signatures
//!
//! - **A method whose return type wraps `Self` in anything but `Option`,
//!   `Result` or a tuple cannot be forwarded**, such as a combinator
//!   `fn buffered(self) -> Buffered<Self>`. The payload's
//!   `Buffered<Payload>` is not the enum's. If the method is provided, leave
//!   it to the trait's default with `#[dispatch(default)]` in a partial impl
//!   block; the default then wraps the enum.
//! - **A dispatch struct whose one field is named cannot forward a method
//!   returning `Option<Self>` or `Result<Self, E>`.** The value is rebuilt
//!   with a path, and no path builds such a struct. Make it a tuple struct;
//!   `-> Self` is rebuilt by the braces either way. The same goes for a
//!   variant whose one field is named and marked; write it as a tuple
//!   variant.
//!
//! ### `impl Trait` in return types
//!
//! - **An enum cannot forward a method that returns `impl Trait`**, at any
//!   depth of the return type. A `match` returns one type, and the attribute
//!   cannot tell that the variants' opaque types are one, even where every
//!   variant reaches a target of one type. `impl Future<Output = T>` is the
//!   exception; it is awaited like an `async fn`. Otherwise, return a
//!   concrete type or a boxed trait object, or write the method in a
//!   partial impl block. A struct forwards any `impl Trait`. A provided
//!   method with a `Self` argument may be left to the trait's default with
//!   `#[dispatch(default)]` there, instead of a `mismatch`.
//!
//! ### Routes and trait objects
//!
//! - **A method that requires `Self: Sized` cannot be forwarded to a
//!   `deref` variant holding a trait object.** A trait object is not
//!   `Sized`. Leave the method to the trait's default with
//!   `#[dispatch(default)]` in a partial impl block. Where the pointer
//!   implements the trait itself, as `Box<dyn Iterator>` does, hold it
//!   without `deref`.
//! - **A route that ends in a method call serves a mutable borrow only
//!   through `via_mut`**, or a method's own route, and a method returning
//!   `Self` cannot use a routed variant. See
//!   [what a route cannot do](#what-a-route-cannot-do).
//! - **A value cannot be moved out of a `deref` variant holding anything but
//!   a `Box`.** A trait method taking `self` gives an error there, and so
//!   does an inherent method's `value` or `field` under `self` or
//!   `self: Box<Self>`, unless the return type borrows it. Hold a `Box`.
//!
//! ### Associated items
//!
//! - **An override, `Name = Type`, converts what a method returns, not an
//!   argument that uses the type.** A method taking `Self::Name`, or a
//!   closure over it, compiles only where every payload's own `Name` is the
//!   override's. Restate the trait with only the methods to forward, or
//!   leave such a method to the default with `#[dispatch(default)]`.
//! - **An enum's generic associated type is declared in a partial impl
//!   block.** It cannot come from the first variant, and no option on the
//!   attribute gives it. A struct takes it from its field's type, except
//!   through a route.
//! - **An associated const is checked only in some cases.** A const of a
//!   type other than an integer, a float, `bool`, `char` or `&str`, or on
//!   an enum with a type or const parameter, is taken from the first
//!   variant without a check. And a variant reached through a field or a
//!   method is left out of the check: its const is not compared.
//! - **A projection through a trait parameter with several bounds,
//!   `F::Value` with `F: Format + Clone`, cannot be qualified at the
//!   enum.** No single bound says which trait `Value` belongs to. The trait
//!   writes `<F as Format>::Value`.
//!
//! ### Partial impl blocks
//!
//! - **An associated type or const a partial impl block declares under
//!   `#[cfg]` is an error.** The forwarded methods convert into the type in
//!   every build. Put the whole block under the `cfg`, with a second block
//!   for the other builds.
//! - **A macro invocation in a partial impl block is not read**, because the
//!   items it writes could not be told from the ones to forward. Write the
//!   items out in the block. One under `#[cfg]` is an error where the `cfg`
//!   is on. Where it is off, the block is read without it, as it is without
//!   any item under `#[cfg]` that cannot be read.
//!
//! ### Structs and conversions
//!
//! - **A dispatch struct with several fields marks the one to forward to**,
//!   with `#[dispatch]`, with `#[dispatch(deref)]` for what a pointer field
//!   points at, or by naming the trait on it, `#[dispatch(Trait)]`.
//!   Otherwise the attribute cannot tell which field is meant. The impl is
//!   still written, around no field, so you get the one error and no
//!   follow-on errors. A bare mark on a pointer is not refused, because the
//!   pointer may implement the trait itself.
//! - **A struct's field lists a trait as the struct's attribute or a
//!   partial impl block's header spells it.** One name spelled two ways,
//!   `Catalog` and `traits::Catalog` or `Encoder` and `Encoder<F>`, is an
//!   error. Spell it as the home does, or import one of two traits of one
//!   name under another name.
//! - **A payload that is an associated type of a type parameter,
//!   `Out(S::Out)`, gets no `From` and no `TryFrom`.** It may be any type,
//!   the enum included, so the impls would overlap the ones `core` writes
//!   for every type. Leave the variant out with `#[dispatch(from = false)]`.
//! - **`from = true` on one of the enum's attributes and `from = false` on
//!   another contradict each other**, and you get the error on the later
//!   one. An `into` target listed twice gets the error on the second. The
//!   conversions are the enum's and are written once.
//!
//! ### Names, lints and the carrier macro
//!
//! - **A mark cannot go under `cfg_attr`.** An attribute macro is handed its
//!   item before a `cfg_attr` inside it is decided, so the mark would be
//!   read in no build, and you get an error on it. A struct marks its field
//!   in every build, with a bare `#[whichever::dispatch]` beside the
//!   trait's attribute under `cfg_attr`. A variant or a method whose mark
//!   differs between builds is written twice under `#[cfg(..)]`.
//! - **A lint attribute on the enum does not reach the impls written next
//!   to it**, as it would not reach a hand-written one. Put `#[allow]` on
//!   the trait method, on a partial impl block, or on the module.
//! - **A private glob import must not bring the trait's own name back into
//!   the trait's module.** An example is `use crate::prelude::*;` at a crate
//!   root whose prelude re-exports a trait declared there. It makes the
//!   carrier macro's re-export ambiguous to rustc
//!   (`ambiguous_import_visibilities`, an error under
//!   `#![deny(future_incompatible)]`). Declare the trait in a module of its
//!   own and re-export it, or import the prelude's items by name there.
//! - **Marking a published `pub` trait adds a macro of the trait's name to
//!   the crate's public macro namespace**: the carrier macro, re-exported
//!   beside the trait. A crate that uses the trait and imports another
//!   macro called `Shape`, such as a derive, then stops compiling: two
//!   named imports give ``error[E0252]: the name `Shape` is defined
//!   multiple times``, and two glob imports give ``error[E0659]: `Shape`
//!   is ambiguous``. In that crate, name the other macro by its path,
//!   `#[derive(shape_derive::Shape)]`, or import it by name beside the
//!   globs. A library that dispatches the trait only in its own crate can
//!   leave it unmarked and restate it crate-privately,
//!   `#[whichever::dispatchable(remote = crate::Shape)] pub(crate) trait Shape { .. }`.
//!   The restatement forwards what it lists, and nothing checks the list
//!   against the trait, so it does not suit a struct that means to forward
//!   every method.
//! - **A trait's items carry at most four distinct `#[cfg]`
//!   predicates.** The carrier macro is written once per combination of
//!   them, so you get the error on the fifth. Combine the predicates, or
//!   move the items under the fifth to a trait of their own.
//! - **One `macro_rules!` may emit a `#[dispatchable]` trait in several
//!   modules only when at least one token of the trait comes from the
//!   invocation**, such as its name or a method's name. A trait written out
//!   in full in the macro's body gets the same carrier macro name at every
//!   invocation, and rustc says it is defined multiple times. Give each
//!   invocation its own name with `#[dispatchable(name = ..)]`.
//!
//! ## Troubleshooting
//!
//! Most errors are rustc reporting on the generated impl, exactly as it
//! would on the same impl written by hand. These are the ones that need a
//! word of explanation:
//!
//! - **"cannot find macro `Shape` in this scope"** at the enum. The trait is
//!   not marked `#[dispatchable]`, or the trait, which carries the macro, is
//!   not imported where the enum is. Mark the trait, or import it.
//! - **A `std::`, `core::` or `alloc::` path in `#[dispatch]`** gets an
//!   error of its own, naming the [`foreign`] restatement or the `remote`
//!   restatement to use instead.
//! - **"cannot find `Trait` in `other`"** (`error[E0433]`) on
//!   `#[dispatch(other::Trait)]`, with "could not find `Trait` in `other`"
//!   in the label. Rust 1.88 words the first line "failed to resolve: could
//!   not find `Trait` in `other`". It means the same as "cannot find macro"
//!   for a trait of another crate: restate it with `remote`.
//! - **The same words on a partial impl block over another crate's enum**
//!   mean the enum has no `#[whichever::dispatch]`, which only its own crate
//!   can add. Or restate the enum with `remote`, as in
//!   [enums from other crates](#enums-from-other-crates).
//! - **An error about the order of attributes.** With `#[async_trait]`,
//!   `#[dispatchable]` goes below it. With `#[typetag::serde]`, it goes
//!   above. Either order the other way round is an error.
//!
//! ## Footprint
//!
//! - The crate is `no_std`, and the generated code names only `core` items.
//! - The minimum supported Rust version is 1.88.
//! - The macros parse with `syn` 3.
//! - The default feature `renamed-dependency` lets a crate rename its
//!   `whichever` dependency. Without it, the macro crate builds
//!   `proc-macro2`, `quote` and `syn` alone.
//!
//! # More documentation
//!
//! Three long-form pages ship with the crate, in the [`docs`] module:
//!
//! - [The tutorial](docs::tutorial): a guided tour, start to finish.
//! - [The glossary](docs::glossary): the words these docs use.
//! - [Troubleshooting](docs::troubleshooting): an error message, its cause
//!   and the fix.
//!
//! The pages in the repository cover the rest at length:
//!
//! - [Limits](https://github.com/shergin/whichever/blob/main/docs/limits.md):
//!   everything the attributes refuse, with the fix.
//! - [When enum dispatch pays](https://github.com/shergin/whichever/blob/main/docs/when-to-use-enum-dispatch.md):
//!   enum, trait object or generic, with the measurements.
//! - [The comparison](https://github.com/shergin/whichever/blob/main/docs/comparison.md):
//!   `whichever` beside the other crates that write this impl.
#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub use whichever_macros::{dispatch, dispatchable};

// The pages are written for reading, and their code is checked where it
// lives, in `examples/tour.rs` and `tests/ui`, so the module is left out
// when rustdoc collects doctests.
#[cfg(all(doc, not(doctest)))]
pub mod docs;
pub mod foreign;

#[doc(hidden)]
pub mod __private {
    pub use whichever_macros::{implement, implement_block};
}
