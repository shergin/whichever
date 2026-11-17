# What is generated

This page shows the code `whichever` writes for you, and what each kind of
trait item turns into.

On this page:

- [What to notice](#what-to-notice)
- [The other things in the expansion](#the-other-things-in-the-expansion)
- [What each trait item becomes](#what-each-trait-item-becomes): the
  table, then
  [associated consts](#associated-consts-and-the-agreement-check),
  [attributes](#inline-and-other-attributes),
  [generic methods](#generic-methods),
  [boxed receivers](#boxed-receivers),
  [pinned receivers](#pinned-receivers), [futures](#futures),
  [`Self` in arguments](#self-in-arguments),
  [`Self` in return types](#self-in-return-types),
  [methods without a receiver](#methods-without-a-receiver),
  [`#[cfg]` on trait items](#cfg-on-trait-items) and
  [`#[cfg]` on variants](#cfg-on-variants)
- [See it yourself](#see-it-yourself)

Start with a trait and an enum over two types that implement it:

```rust
// In the crate that owns the trait.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
}

// In any crate that owns an enum over implementors.
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

`AnyShape` now implements `Shape`. The impl is the one you would write by
hand:

```rust
impl Shape for AnyShape {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            AnyShape::Circle(inner) => <Circle as Shape>::area(inner),
            AnyShape::Square(inner) => <Square as Shape>::area(inner),
        }
    }
    #[inline]
    fn name(&self) -> &'static str {
        match self {
            AnyShape::Circle(inner) => <Circle as Shape>::name(inner),
            AnyShape::Square(inner) => <Square as Shape>::name(inner),
        }
    }
}

impl From<Circle> for AnyShape {
    #[inline]
    fn from(value: Circle) -> Self {
        Self::Circle(value)
    }
}

// And the same `From` impl for `Square`.
```

## What to notice

- **One `match` per method**, with one arm per variant.
- **Every call is fully qualified**: `<Circle as Shape>::area(inner)`, not
  `inner.area()`. So if `Circle` also has an inherent method called
  `area`, it never takes over from the trait's method.
- **Provided methods are forwarded too.** `name` has a default body in the
  trait, but the enum still calls each payload's `name`. If `Circle`
  overrides `name`, you get the override, not the default.
- **Every method is `#[inline]`**, so the optimiser can see through the
  `match` into the payload's method, even across crates.
- **Nothing else is added**: no trait object, no closure, no conversion
  the signature does not ask for.
- **The `From` impls** let you write `AnyShape::from(circle)` or
  `circle.into()`. You can turn them off, or add `TryFrom` and more; see
  [Options](options.md).

## The other things in the expansion

Run `cargo expand` and you see a few more things. None of them adds any
work at run time.

- The impl is marked `#[automatically_derived]`. This tells Clippy and
  coverage tools that the code is generated.
- The bindings in the arms are called `__inner`, `__same0` and the like,
  not `inner`. They have `macro_rules`-style hygiene, so they never collide
  with a name the trait uses.
- A `const _` block next to the impl holds the
  [agreement checks](glossary.md#agreement-check): compile-time checks
  that every variant has the same associated types and consts. They
  compile to nothing.
- The enum gets its own crate-local carrier macro, re-exported under the
  enum's name. It holds the enum's definition, for the enum's impl blocks
  (an [inherent block](inherent-methods.md) or a
  [partial impl block](partial-impl-blocks.md)) to read.

## What each trait item becomes

A quick lookup. Where a row needs more than a line, it links to a section
after the table.

| Trait item | In the enum's impl |
| --- | --- |
| `type X;` | `type X = <FirstVariant as Trait>::X;`, or the `X = Type` option. Every other variant is checked to agree, and a disagreement is an error on that variant. |
| `const N: T;` | `const N: T = <FirstVariant as Trait>::N;`, or the `N = value` option. Some consts are checked to agree on every variant; see [the agreement check](#associated-consts-and-the-agreement-check). |
| method with a receiver | A `match` over the variants calling `<Payload as Trait>::method(payload, args..)`. It is `#[inline]`; see [attributes](#inline-and-other-attributes). |
| method with a default body | The default is ignored: the method is forwarded like any other. |
| generic method | Forwarded with its generic parameters spelled out when no argument fixes them; see [generic methods](#generic-methods). |
| `self: Box<Self>` | Works for `deref` variants that hold a `Box`; see [boxed receivers](#boxed-receivers). |
| `self: Pin<&Self>`, `self: Pin<&mut Self>` | Unpinned, matched and pinned again. Every payload must be `Unpin`; see [pinned receivers](#pinned-receivers). |
| `async fn` | Each arm is awaited. |
| `unsafe fn` | Each call is wrapped in `unsafe { }`. |
| `unsafe trait` | An `unsafe impl`. |
| `-> impl Future<Output = T>` | One `async move` block around the `match`, each arm awaited; see [futures](#futures). |
| `-> Pin<Box<dyn Future<Output = T> + ..>>`, as `#[async_trait]` writes it | Forwarded as it is; see [futures](#futures). |
| any other `-> impl Trait` | An error on an enum. Forwarded as it is on a [struct](structs.md). |
| argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`. Different variants return the `mismatch` expression; see [`Self` in arguments](#self-in-arguments). |
| return `Self` | Wrapped back into the same variant; see [`Self` in return types](#self-in-return-types) for `Option<Self>`, `Result<Self, E>` and tuples. |
| method without a receiver | Left to the trait's default body, or written by you in a partial impl block. An error if it has neither; see [methods without a receiver](#methods-without-a-receiver). |
| `#[cfg]` on a method or an associated item | Decided in the trait's crate; see [`#[cfg]` on trait items](#cfg-on-trait-items). |
| `#[cfg]` on a variant | Carried onto the variant's arm, and decided where the enum is; see [`#[cfg]` on variants](#cfg-on-variants). |
| `#[deprecated]` on a variant | Arms and conversions written under `#[allow(deprecated)]`. |
| `#[deprecated]` on a method or an associated item | Forwarded under `#[allow(deprecated)]`. |
| `#[allow(..)]` on a method or an associated item | Carried onto the generated item. |

### Associated consts and the agreement check

Each associated const comes from the first variant, unless you set it on
the enum with `N = value` (see [Options](options.md)). For some consts,
you also get a compile-time check that every other variant has the same
value. You get the check when both of these are true:

- the const's type is an integer, a float, `bool`, `char` or `&str`;
- the enum has no type or const parameter (lifetime parameters do not
  matter here).

When a variant disagrees, the error is on that variant. Every variant
that disagrees gets its own error, not only the first.

### `#[inline]` and other attributes

Every generated method gets `#[inline]`. If the trait method has its own
`#[inline(..)]`, such as `#[inline(always)]` or `#[inline(never)]`, that
one is used instead. If you write or mark the method in a
[partial impl block](partial-impl-blocks.md), it carries the attributes
you put on it there instead.

You can write `#[inline(..)]` on a required trait method, one without a
body, to choose how its forwarding methods are inlined. The carrier macro
keeps it for them, and `#[dispatchable]` takes it off the trait's own
method. That method has no body to inline, and rustc warns about the
attribute there ("the `inline` attribute cannot be used on required trait
methods", a future hard error that Clippy already denies). A provided
method keeps its `#[inline(..)]`, which is its own body's too.

One exception: an `#[inline(..)]` written under `cfg_attr` is not carried.
Its predicate belongs to the trait's crate, and the forwarding method is
written in the enum's crate, where it cannot be decided.

The other attributes that are carried over:

- **`#[deprecated]` on a trait method or associated item.** The forwarding
  is written under `#[allow(deprecated)]`, as you would write it by hand.
  Your callers still see the deprecation, through the trait's item.
- **`#[deprecated]` on a variant.** Its arms and its conversions are
  written under `#[allow(deprecated)]`. The deprecation still reaches the
  code that names the variant.
- **`#[allow(..)]` on a trait method or associated item** is copied onto
  the generated item. The generated item repeats the signature that the
  lint was allowed on, so it needs the same `allow`. A `warn`, `deny` or
  `expect` stays with the trait's crate and is not copied. Neither is an
  `#[allow]` that another macro added.

### Generic methods

A generic method is forwarded with its type and const parameters spelled
out, but only when no argument fixes them:

```rust
#[whichever::dispatchable]
pub trait Source {
    fn parse<T: FromStr>(&self) -> Option<T>;
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32;
}
```

The arms are `<Text as Source>::parse::<T>(inner)`, because nothing else
tells the compiler what `T` is, and `<Text as Source>::apply(inner, f)`,
because the argument `f` already fixes `F`.

#### Renamed generic parameters

Sometimes a method's generic parameter has the same name as something
the impl already gives a meaning to. Then the two would clash, so the
method's parameter is renamed, as you would rename it by hand:

```rust
#[whichever::dispatchable]
pub trait Make {
    fn make<T: Default>(&self) -> T;
}

#[whichever::dispatch(Make)]
pub enum Holder<T> {
    Plain(Plain),
    Wrapped(Wrapped<T>),
}
```

Inside `impl<T> Make for Holder<T>`, `T` already means the enum's
parameter. So the generated method is `fn make<T2: Default>(&self) -> T2`.

The new name adds the first number that neither the impl nor the trait
already uses: `T2`, then `T3`, and so on. Lifetimes and const parameters
work the same way, giving `'a2` and `N2`.

A method's parameter is renamed when its name is one of these:

- a parameter of the enum, as above, or of a partial impl block's
  header: `fn eq<I>(self, other: I)` over `struct Tagged<I>` becomes
  `fn eq<I2>(self, other: I2)`;
- a name in the header's bounds;
- a payload's type in an arm: `fn pick<Square>` over a payload `Square`
  becomes `fn pick<Square2>`;
- an argument given to the trait, as in `Encoder<Json>` (see
  [generic traits](options.md#generic-traits)).

Some things do not cause a rename:

- A name that follows `::` is no reason to rename. `IntoIter` in a payload
  `std::vec::IntoIter<u32>` does not rename a parameter `IntoIter`.
- A method that a partial impl block writes or marks keeps the names you
  wrote on it.

The trait method's own `mismatch` expression follows the new names:
`#[dispatch(mismatch = T::default())]` becomes `T2::default()` in the
renamed method.

### Boxed receivers

For a method that takes `self: Box<Self>`, the enum is moved out of its
box, and the method is called with the variant's own `Box<Payload>`. So
the variant must hold a `Box` under `#[dispatch(deref)]`:

```rust
#[whichever::dispatchable]
pub trait Job {
    fn finish(self: Box<Self>) -> u32;
}

#[whichever::dispatch(Job)]
pub enum AnyJob {
    #[dispatch(deref)]
    Boxed(Box<Ready>),
}
```

A plain variant, `Ready(Ready)`, is an error: to call the method, the arm
would have to box the payload again, and that would allocate.

### Pinned receivers

For `self: Pin<&Self>` and `self: Pin<&mut Self>`, the enum is unpinned,
matched, and the payload is pinned again before the call. Pinning the
payload again with `Pin::new` needs it to be `Unpin`, so every payload
has to be `Unpin`.

### Futures

`async fn` and `-> impl Future<Output = T>` both work on an enum.

- **`async fn`**: each arm is awaited inside the generated `async fn`.
- **`-> impl Future<Output = T>`**: the whole `match` goes into one
  `async move` block, and every arm is awaited inside it, as for an
  `async fn`. Each payload returns a different future type, and this is
  how they meet in one. `Send` and any other bound on the future are
  checked against that block.
- **On a [struct](structs.md)**, there is one arm, so the field's future
  is returned as it is, unless its output is rewrapped (a `Self`) or
  converted (an [override](options.md#name--type-override-an-associated-type)).

`#[async_trait]` writes `async fn` as
`-> Pin<Box<dyn Future<Output = T> + ..>>`. That is one type for every
arm, so it is forwarded as it is. When `T` has to be rewrapped or
converted, every arm is awaited in one `async move` block, which is
boxed and pinned again with the `Box` the signature names, as
`#[async_trait]` itself does.

Any other `-> impl Trait` is an error on an enum, because each arm would
return a different hidden type and a `match` needs one. A struct has one
arm, so it forwards any `impl Trait`.

### `Self` in arguments

A method can take another value of the enum as `Self`, `&Self` or
`&mut Self`. The generated method matches `self` and the argument
together. When both hold the same variant, the payloads meet in the
payload's method:

```rust
#[whichever::dispatchable]
pub trait Shape {
    #[dispatch(mismatch = false)]
    fn congruent(&self, other: &Self) -> bool;
}
```

```rust
fn congruent(&self, other: &Self) -> bool {
    match (self, other) {
        (AnyShape::Circle(inner), AnyShape::Circle(same0)) => {
            <Circle as Shape>::congruent(inner, same0)
        }
        (AnyShape::Square(inner), AnyShape::Square(same0)) => {
            <Square as Shape>::congruent(inner, same0)
        }
        _ => false,
    }
}
```

When they hold different variants, there is no payload method to call.
The method returns the [mismatch](glossary.md#mismatch) expression
instead, here `false`. You write it on the trait method, as above, or on
the enum; [`mismatch`](options.md#mismatch-what-to-return-when-two-variants-differ)
has the details.

### `Self` in return types

A method that returns `Self` gets the payload's result wrapped back into
the same variant:

```rust
// fn scaled(&self, factor: u32) -> Self;
AnyShape::Circle(inner) => AnyShape::Circle(<Circle as Shape>::scaled(inner, factor)),
```

A few other return types work too. In them, `Self` is wrapped back into
the variant, and an [overridden](glossary.md#override) associated type
is converted with `Into`:

- `Option<Self>` and `Result<Self, E>` are mapped back into the variant.
  They are found under any path, so `io::Result<Self>` works too.
- A tuple holding `Self` or an overridden associated type, such as
  `(Self, u32)`, is taken apart and built again around whichever of the
  two it holds: `Self` wrapped back into the variant, an overridden type
  converted with `Into`.

`Self` anywhere else in a return type is an error, because the rewrap
would need a closure. That covers:

- `Self` inside any other type, such as `Vec<Self>` or `Buffered<Self>`;
- a tuple under an `Option` or a `Result`, such as `Option<(Self, u32)>`;
- `Option<Self>` or `Result<Self, E>` on a struct whose one field is
  named, `struct Named { shape: Square }`, or on a variant whose one
  field is named and marked, `Named { #[dispatch] shape: Square }`. The
  value is rebuilt with a path, and no path builds such a value. The
  tuple form, `struct Named(Square);` or `Named(Square)`, works.

If such a method is a provided method, you have a way out: leave it to
the trait's default body with `#[dispatch(default)]` in a
[partial impl block](partial-impl-blocks.md).

### Methods without a receiver

A method without `self`, such as `fn new() -> Self`, has no value to
match on. It is left to the trait's default body if it has one. If it does
not, write it yourself in a partial impl block,
`#[whichever::dispatch] impl Trait for Enum { .. }`. When a method has
neither, you get an error.

### `#[cfg]` on trait items

A `#[cfg]` on a trait method or associated item is decided in the trait's
crate, with that crate's features and settings, not the enum's.

This works because the carrier macro is written once for each
combination of the trait's distinct `cfg` predicates, each copy under its
own `cfg`. So the generated item exists exactly when the trait's item
does, in a build of the trait's crate.

### `#[cfg]` on variants

A `#[cfg]` on a variant is copied onto that variant's arms, and decided
where the enum is:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Circle(Circle),
    #[cfg(test)]
    Mock(MockShape),
}
```

The fine print:

- **Errors about a variant follow its `cfg`.** When a method cannot be
  forwarded to a variant, the error is that arm's result, so it sits
  under the variant's `cfg` too. A variant that is compiled out is never
  an error, and each variant the method cannot reach gets its own error,
  naming it.
- **Associated types and consts** come from the first variant that is
  always compiled in and whose target has a type the attribute can
  name. So a variant under `#[cfg]` is passed over, and so is one
  reached through a field or a method.
- **Every variant under `#[cfg]`.** A build may leave no variant at all.
  For that case, a method whose receiver is a reference matches
  `match *self` with `ref` bindings, so the empty build still compiles, as
  a hand-written one does.

## See it yourself

To see the exact code for your own enum, install
[`cargo-expand`](https://github.com/dtolnay/cargo-expand) and run it on
the crate that holds the enum:

```bash
cargo expand --lib path::to::module
```

[Options](options.md) lists everything you can tell the attribute, and
[Limits](limits.md) lists what it refuses, each with what to do
instead.
