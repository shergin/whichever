# A tour of `whichever`

This page walks you through `whichever` one feature at a time. You need to
know Rust traits and enums, and about fifteen minutes. Each step shows the
everyday use of one feature and links to the details, so you can stop at
any step and still be right.

The whole tour is one program, [`examples/tour.rs`][tour-rs],
with one module per step. Every snippet below is copied from it, and each
step checks its claims with `assert!`. Run it from a clone of the
repository:

```bash
cargo run --example tour
```

Most steps use shapes: a `Shape` trait, a `Circle` and a `Square`. Words
in **bold** are defined in the [glossary][glossary].

## 1. Write your first dispatch enum

Add the crate:

```bash
cargo add whichever
```

Put `#[whichever::dispatchable]` on the trait, and
`#[whichever::dispatch(Shape)]` on an enum whose variants each hold one
type that implements it:

```rust
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle(pub u32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Square(pub u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
    fn name(&self) -> &'static str {
        "circle"
    }
}

// `Square` keeps the trait's default `name`.
impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

Now `AnyShape` is a `Shape`:

```rust
let shapes: Vec<AnyShape> = vec![Circle(1).into(), Square(2).into()];
let total: u32 = shapes.iter().map(|shape| shape.area()).sum();
assert_eq!(total, 7);
```

An enum with `#[dispatch]` on it is a **dispatch enum**, and the value a
variant holds, the `Circle` in `Circle(Circle)`, is its **payload**. The
attribute writes the impl you would write by hand, nothing more (shown
here with tidier names for the bindings):

```rust
impl Shape for AnyShape {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            AnyShape::Circle(inner) => <Circle as Shape>::area(inner),
            AnyShape::Square(inner) => <Square as Shape>::area(inner),
        }
    }
    // `name` is forwarded the same way.
}
```

One `match` per method, one call per variant, and no trait object or
allocation anywhere. Passing the call on like this is called
**forwarding**. You also get `From<Circle>` and `From<Square>`, which is
why `Circle(1).into()` works above. [What is
generated][what-is-generated] shows the full expansion.

## 2. Put the trait and the enum in different crates

Nothing changes. Both crates depend on `whichever`, and the enum's crate
imports the trait. In the tour, two modules stand in for the two crates:

```rust
mod shapes_crate {
    #[derive(Debug, PartialEq)]
    pub struct Size {
        pub width: u32,
        pub height: u32,
    }

    #[whichever::dispatchable]
    pub trait Measure {
        fn size(&self) -> Size;
    }
}

mod app_crate {
    // `Measure` brings its carrier macro along. `Size` is imported
    // because the generated impl writes it as the trait spells it.
    use super::shapes_crate::{Measure, Size};
    use crate::first_enum::{Circle, Square};

    #[whichever::dispatch(Measure)]
    pub enum Measured {
        Circle(Circle),
        Square(Square),
    }
}
```

In a real project the `use` line would read `use shapes::{Measure, Size};`.

How does the enum's crate see the trait's methods? `#[dispatchable]`
leaves a hidden `macro_rules!` macro next to the trait, holding its
signatures: the **carrier macro**. It has the trait's name, so importing
the trait imports the macro too, and `#[dispatch]` reads the signatures
from it. Nothing is cached between macro expansions. See the
[glossary][glossary-carrier-macro] and
[Compatibility][compatibility], which also explains why both crates
should use the same minor version of `whichever` before 1.0.

The one thing to remember: the types the trait's signatures mention,
like `Size` here, must be in scope where the enum is, just as they would
be for an impl you wrote by hand.

## 3. Let a payload override a provided method

A **provided method** is a trait method with a default body, like `name`.
`Circle` overrides it and `Square` does not. The enum forwards `name`
like any other method, so each payload's own version is what runs:

```rust
let circle = AnyShape::from(Circle(1));
let square = AnyShape::from(Square(1));
assert_eq!(circle.name(), "circle");
assert_eq!(square.name(), "shape");
```

The default body is not used at the enum level, so an override is never
hidden by it.

## 4. Use `Self` in a signature

A method can return `Self`. The enum wraps the payload's result back into
the same variant. A method can also take `&Self`, which raises a
question: what if `self` and the argument hold different variants? You
say what to return with `mismatch`:

```rust
#[whichever::dispatchable]
pub trait Resize {
    fn scaled(&self, factor: u32) -> Self;

    /// Two different kinds of shape are never congruent.
    #[dispatch(mismatch = false)]
    fn congruent(&self, other: &Self) -> bool;
}

#[whichever::dispatch(Shape)]
#[whichever::dispatch(Resize)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

```rust
let square = AnyShape::from(Square(2));
let bigger = square.scaled(3);
assert!(matches!(bigger, AnyShape::Square(Square(6))));
assert_eq!(bigger.area(), 36);

let circle = AnyShape::from(Circle(2));
assert!(square.congruent(&AnyShape::from(Square(2))));
assert!(!square.congruent(&circle));
```

A square scaled is still a `Square` variant. Two squares are compared by
`Square::congruent`, and a square and a circle get the **mismatch**
value, `false`. Notice also that one enum can dispatch several traits:
repeat the attribute, one trait each.

- `Option<Self>` and `Result<Self, E>` are wrapped back too.
- If the trait is not yours to change, put the value on the enum
  instead: `#[whichever::dispatch(Resize, mismatch(congruent = false))]`.
- If you forget the value, the error tells you both places:

```text
error: `congruent` takes `Self` arguments; give the result for two different variants with `#[dispatch(mismatch = <expr>)]` on the trait method or `mismatch(congruent = <expr>)` on the enum
```

[What is generated][what-is-generated-items] and [Options][options] have
the details.

## 5. Use associated types and consts

Shapes have no natural associated type, so this step switches to storage
backends. Each backend has a block size and its own error type:

```rust
#[whichever::dispatchable]
pub trait Storage {
    const BLOCK_SIZE: usize;
    type Error;
    fn load(&self, key: &str) -> Result<Vec<u8>, Self::Error>;
}

#[whichever::dispatch(Storage, Error = StorageError)]
pub enum AnyStorage {
    Memory(Memory),   // Error = Infallible
    Network(Network), // Error = Timeout
}
```

```rust
assert_eq!(AnyStorage::BLOCK_SIZE, 4096);
let network = AnyStorage::from(Network);
assert_eq!(network.load("greeting"), Err(StorageError::Timeout));
```

There are two ways the enum gets an associated item:

- **From the first variant.** `BLOCK_SIZE` is `Memory`'s, and a check at
  compile time makes sure every other variant has the same value. This
  is the **agreement check**: if `Network` said 512, you would get an
  error on the `Network` variant.
- **Fixed on the enum.** The two backends fail in different ways, so
  `Error = StorageError` sets the enum's type. This is an **override**.
  Each payload's own error converts into `StorageError` with `Into`, so
  `StorageError` needs `From<Infallible>` and `From<Timeout>`.

A const can be fixed the same way, `NAME = value`. See
[Options][options].

## 6. Convert between the enum and its payloads

You already have `From<Payload>` for every variant. Options on the
attribute add more conversions:

```rust
#[derive(Debug)]
#[whichever::dispatch(Shape, try_from = true)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

```rust
let shape: AnyShape = Circle(2).into();
assert_eq!(Circle::try_from(shape).unwrap(), Circle(2));

// A failed `try_from` hands the enum back as the error.
let shape: AnyShape = Square(2).into();
let back = Circle::try_from(shape).unwrap_err();
assert!(matches!(back, AnyShape::Square(Square(2))));
```

`try_from = true` adds `TryFrom<AnyShape> for Circle` and for `Square`.
The other conversion options:

- `from = false` turns the `From` impls off, for the whole enum or, on a
  variant, for that variant alone.
- `into = Target` adds `From<AnyShape> for Target`, when every payload
  converts into `Target`.

[Options][options] lists them all.

## 7. Hold a payload behind a pointer

A variant can hold a `Box<T>`, an `Rc<T>` or a `&T`. Mark it
`#[dispatch(deref)]`, and the trait is called on what the pointer points
at:

```rust
#[whichever::dispatch(Shape)]
pub enum Held<'a> {
    Circle(Circle),
    #[dispatch(deref)]
    Boxed(Box<Square>),
    #[dispatch(deref)]
    Shared(Rc<Square>),
    #[dispatch(deref)]
    Borrowed(&'a Square),
}
```

The same trick keeps the set of types open. Give the enum one variant
that holds a `Box<dyn Shape>`:

```rust
#[whichever::dispatch(Shape)]
pub enum OpenShape {
    Circle(Circle),
    Square(Square),
    #[dispatch(deref)]
    Other(Box<dyn Shape>),
}
```

```rust
let other = OpenShape::Other(Box::new(Triangle(4)));
assert_eq!((other.name(), other.area()), ("triangle", 8));
```

`Circle` and `Square` stay inline and fast. `Other` takes any shape at
all, from any crate, for the cost of one vtable call. For this the trait
must be usable as `dyn Shape`: it must be dyn compatible (what Rust used
to call object safe).

`deref` is the simplest **route**: the steps from the payload to the
value the trait is called on. A route can also reach a field or what a
method returns; [Reaching the target][routes] shows how.

## 8. Give the enum methods of its own

Sometimes only some variants can answer a question. Put `#[dispatch]` on
an inherent `impl` block, and mark each method with the variants to
forward to:

```rust
#[whichever::dispatch]
impl AnyShape {
    /// Only circles have a radius.
    #[dispatch(Circle, with = Some)]
    pub fn radius(&self) -> Option<u32> {
        None
    }

    #[dispatch(*)]
    pub fn describe(&self) -> String;
}
```

```rust
let circle = AnyShape::from(Circle(2));
let square = AnyShape::from(Square(3));
assert_eq!(circle.radius(), Some(2));
assert_eq!(square.radius(), None);
assert_eq!(square.describe(), "square of side 3");
```

`radius` calls `Circle::radius` and wraps the result in `Some`. Every
other variant runs the body, `None`: that body is the **fallback**.
`describe` is forwarded to every variant (`*`), so it needs no body.
Here `Circle` and `Square` have inherent `radius` and `describe` methods;
no trait is involved. See [Inherent methods][inherent-methods].

## 9. Write one trait method yourself

Some trait methods cannot be forwarded: a constructor has no `self` to
match on. Others you want to handle differently at the enum. Write those
in a trait impl block with `#[dispatch]` on it, and the rest of the trait
is forwarded around them:

```rust
#[whichever::dispatchable]
pub trait Sides {
    /// The smallest shape of this kind.
    fn unit() -> Self;
    fn sides(&self) -> u32;
    fn summary(&self) -> String {
        format!("{} sides", self.sides())
    }
}

#[whichever::dispatch]
impl Sides for AnyShape {
    fn unit() -> Self {
        AnyShape::Square(Square::unit())
    }
    fn summary(&self) -> String {
        format!("a shape with {} sides", self.sides())
    }
}
```

```rust
let unit = AnyShape::unit();
assert!(matches!(unit, AnyShape::Square(Square(1))));
assert_eq!(unit.sides(), 4);
assert_eq!(unit.summary(), "a shape with 4 sides");
```

This is a **partial impl block**: you wrote `unit` and `summary`, and
`sides` was forwarded. Since the block is the impl, the enum does not
list `Sides` in its own attribute. See [Partial impl
blocks][partial-impl-blocks].

## 10. Forward a trait you do not own

`#[dispatch]` needs the trait's carrier macro, and traits from `std` do
not have one. For the common ones, `whichever::foreign` has one ready.
Here an error enum gets `Display` without a hand-written `match`:

```rust
#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
pub enum ShapeError {
    Parse(ParseIntError),
    TooBig(TooBig),
    #[dispatch(from = false)]
    Unknown(String),
}
```

```rust
let too_big = ShapeError::from(TooBig(1000));
let unknown = ShapeError::Unknown("hexagon".to_string());
assert_eq!(too_big.to_string(), "1000 is too big");
assert_eq!(unknown.to_string(), "hexagon");
```

Each variant prints with its payload's own `Display`. The `String`
variant opts out of `From`, so a plain string does not turn into a
`ShapeError` by accident.

For any other trait, copy its signatures once into a trait marked
`#[whichever::dispatchable(remote = path::to::Trait)]`: a
**restatement**. [Traits and enums from other crates][restatements]
shows how, and lists what `whichever::foreign` holds.

## 11. Wrap the enum in a struct

A dispatch enum is often private, behind a public struct. Put
`#[dispatch(Shape)]` on the struct and mark the field to forward to:

```rust
#[whichever::dispatch(Shape)]
pub struct Labelled {
    #[dispatch]
    shape: AnyShape,
    label: &'static str,
}
```

```rust
let labelled = Labelled {
    shape: Square(3).into(),
    label: "tile",
};
assert_eq!((labelled.area(), labelled.name()), (9, "shape"));
```

Every method is forwarded to `shape`, provided methods included. The
field can be any type that implements the trait, not only an enum. See
[Forwarding from a struct field][structs].

## 12. Forward `async fn`

An `async fn` in the trait works with no extra setup. Each arm of the
`match` awaits the payload's future:

```rust
#[whichever::dispatchable]
trait Render {
    async fn render(&self) -> String;
}

#[whichever::dispatch(Render)]
enum Drawing {
    Circle(Circle),
    Square(Square),
}
```

The tour runs `Drawing::from(Circle(2)).render()` with
`pollster::block_on` and gets `<circle r=2/>`. A method that returns
`impl Future<Output = T>` is awaited the same way. See [What is
generated][what-is-generated-items].

## 13. Read an error when it says no

Some trait methods cannot be forwarded by a `match`. When you hit one,
`whichever` promises three things:

- You get a one-sentence error on the trait item or variant at fault.
- The error tells you the fix, where there is one.
- The rest of the enum still compiles, so you see the error for the thing
  to fix, and no wall of follow-on errors.

Here is a method that returns `impl Iterator`:

```rust
#[whichever::dispatchable]
pub trait Edges {
    fn edges(&self) -> impl Iterator<Item = u32>;
}
```

Each variant would return a different hidden type, and a `match` needs
one. The error, on the return type:

```text
error: a return-position `impl Trait` cannot be dispatched by a `match` because every variant returns a different opaque type; return a concrete type or a boxed trait object, or `impl Future<Output = T>`, which is awaited like an `async fn`
```

Follow its advice and return a concrete type:

```rust
#[whichever::dispatchable]
pub trait Edges {
    fn edges(&self) -> Vec<u32>;
}

#[whichever::dispatch(Edges)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

Everything the attribute refuses like this is a **limit**, listed in
[Limits][limits].
[Troubleshooting][troubleshooting] goes the other way: start from the
error message and find the cause and the fix.

## Where to go next

- **See exactly what the attribute writes:**
  [What is generated][what-is-generated].
- **Look up an option:** [Options][options].
- **Reach a field, or call through a pointer:**
  [Reaching the target][routes].
- **Forward to only some variants:**
  [Inherent methods][inherent-methods] and
  [Partial impl blocks][partial-impl-blocks].
- **Forward a trait or an enum from another crate:**
  [Restatements][restatements].
- **Wrap the enum in a struct:** [Structs][structs].
- **Know what it cannot do:** [Limits][limits] and
  [Troubleshooting][troubleshooting].
- **Decide between an enum and a trait object:**
  [When enum dispatch pays][when-to-use-enum-dispatch].
- **Compare it with other crates:** [Comparison][comparison], or
  [Moving from `enum_dispatch`][moving-from-enum-dispatch].
- **Check versions and dependencies:**
  [Compatibility][compatibility-footprint].
- **Look up a word:** [Glossary][glossary].

<!-- The links are reference-style because src/docs.rs includes this page in the crate docs and gives each label a target there. -->
[comparison]: comparison.md
[compatibility]: compatibility.md
[compatibility-footprint]: compatibility.md
[glossary]: glossary.md
[glossary-carrier-macro]: glossary.md#carrier-macro
[inherent-methods]: inherent-methods.md
[limits]: limits.md
[moving-from-enum-dispatch]: moving-from-enum-dispatch.md
[options]: options.md
[partial-impl-blocks]: partial-impl-blocks.md
[restatements]: restatements.md
[routes]: routes.md
[structs]: structs.md
[tour-rs]: ../examples/tour.rs
[troubleshooting]: troubleshooting.md
[what-is-generated]: what-is-generated.md
[what-is-generated-items]: what-is-generated.md
[when-to-use-enum-dispatch]: when-to-use-enum-dispatch.md
