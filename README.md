# whichever

[![crates.io](https://img.shields.io/crates/v/whichever.svg)](https://crates.io/crates/whichever)
[![docs.rs](https://docs.rs/whichever/badge.svg)](https://docs.rs/whichever)
[![CI](https://github.com/shergin/whichever/actions/workflows/ci.yml/badge.svg)](https://github.com/shergin/whichever/actions/workflows/ci.yml)

Implements a trait for an enum by forwarding to whichever variant it holds.

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

`AnyShape` now implements `Shape` with one `match` per method, each arm a
fully qualified call into the variant's payload, and gets `From<Circle>` and
`From<Square>`:

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
```

That is the code a careful hand would write: no trait object, no closure,
no conversion, and a variant's override of a default method is never hidden
by the default.

## Documentation

- This file: what the attributes generate, every option, and the limits.
  The API reference is on [docs.rs](https://docs.rs/whichever).
- [When enum dispatch pays](docs/when-to-use-enum-dispatch.md): choosing
  between an enum, a trait object and a generic, with the measurements
  behind each piece of advice.
- [Benchmarks](benchmarks/README.md): what is compared with what, how, and
  how to run it; [results](benchmarks/RESULTS.md) from the last run.

## Why another one

An enum over the implementors of a trait is the cheapest way to hold "one
of these": no allocation, no vtable, and the compiler sees every arm. The
forwarding impl is the price, paid again for every method and every variant.

`enum_dispatch` made it automatic, but it keeps the trait's definition in a
process-global cache inside the proc-macro server. That is why it cannot see
a trait from another crate (the enum compiles with no impl and the first
call fails), panics on associated types and receiver-less methods, and
loses the spans of its errors. `whichever` keeps nothing between
expansions: `#[dispatchable]` exports a hidden macro carrying the trait's
tokens, re-exported under the trait's own name, and `#[dispatch]` calls it.
The trait and the enum may live in different crates, in any order, and the
shapes real traits have (associated types, `&Self` arguments, generic
traits, several traits on one enum) dispatch too.

| | whichever | enum_dispatch 0.3 | ambassador 0.5 | portrait 0.3 | enum_delegate 0.2 | delegation 0.5 | static-dispatch 0.3 | delegate 0.13 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Trait and enum in different crates | yes | no, silently | yes | yes | yes | yes | yes | yes, signatures re-listed |
| Traits of `std` or a crate you do not own | yes, `Display`, `io::Read` and more shipped; others restated once | no | yes, restated | - | - | - | - | yes, signatures re-listed |
| Default methods forwarded | yes | yes | yes | yes | yes | yes | yes | if listed |
| `Self`, `&Self`, `&mut Self` arguments | yes, with a declared fallback | no | no | no | panics at runtime | no | no | no |
| `-> Self` | yes | yes | no | no | yes | no | no | with `#[into]` |
| `-> Option<Self>`, `-> Result<Self, E>` | yes | no | - | - | - | - | - | - |
| Associated types | yes | no | yes | no | yes | no | no | by hand |
| Differing associated types unified | yes, through `Into` | no | no | no | yes, wrapper enum | no | no | by hand |
| Associated consts | yes, checked to agree or fixed on the enum | no | no | no | no | no | no | by hand |
| Methods without a receiver | with a default body, or written by hand next to the forwarded ones | no | no | no | no | no | no | - |
| Some methods written by hand, the rest forwarded | yes | no | no | no | no | no | no | yes, by listing the rest |
| `async fn` | yes | yes | yes | no | no | no | yes | yes |
| Generic trait at a concrete instantiation | yes | no | yes | no | no | yes | no | by hand |
| Const generics on the trait | yes | - | - | - | - | - | - | - |
| Generic methods | yes | yes | yes | yes | no | yes | yes | yes |
| Lifetime-generic trait and enum | yes | no | yes | no | no | yes | no | by hand |
| Several traits on one enum | yes | yes | yes | yes | no | no | yes | yes |
| Payload with a same-named inherent method | safe | safe | safe | safe | safe | safe | safe | calls it, unless `#[through]` |
| `Box<T>`, `Rc<T>`, `&T` payloads | yes, `#[dispatch(deref)]` | no | no | no | no | no | no | yes |
| `-> impl Trait` | `impl Future<Output = T>` only | no | no | no | no | no | no | no |
| `From<Payload>` impls | yes, opt-out; `TryFrom<Enum>` opt-in | yes, plus a forced `TryInto` | no | no | yes | no | no | no |
| Inherent methods forwarded to some variants, with a fallback | yes | no | no | no | no | no | no | by hand |

Probed on 2026-11-01 against the versions shown, with the trait in one
crate and the enum in another, on stable Rust. A dash means the case was
not probed for that crate. The `whichever` column is what `tests/` checks.

## Performance

The `match` the attribute writes is the one a careful hand writes, and so
is the machine code. [`benchmarks/`](benchmarks/README.md) compares
`whichever` with a hand-written impl, with `Box<dyn Trait>` and with the
enums of `enum_dispatch`, `ambassador`, `enum_delegate`, `delegation` and
`delegate`, calling from a crate other than the enum's, without LTO, over
collections of 2 to 32 variants in uniform, cyclic and random layouts, with
method bodies the optimiser can inline and bodies it cannot. From the last
run, on an Apple M1 Pro ([full tables](benchmarks/RESULTS.md)):

- `whichever` is within 1.4% of the hand-written `match` in every cell of
  the main matrix, two families by three layouts by five variant counts, and
  the functions it generates are identical to the hand-written ones
  instruction for instruction.
- Over a random 8-variant collection, one `&self` call costs 1.7 ns per
  element against 2.3 ns through a `Box<dyn Trait>` when the method bodies
  can be inlined, and 2.1 ns against 2.3 ns when they cannot. Most of the
  enum's advantage is what the optimiser gets to see, not the dispatch.
- Where the payload's place matters, the gap is wide: 1.7 ns against 8.1 ns
  per element over 4 Mi scattered elements, and a collection is built 13x
  faster without a box per element.
- A generated impl without `#[inline]` (`enum_delegate`, `delegation`) pays
  an extra out-of-line call across the crate boundary, from 15% to 4x
  depending on the method and the variant count.

Whether an enum is the right choice for a given program, and how to get
these numbers out of it, is the subject of
[docs/when-to-use-enum-dispatch.md](docs/when-to-use-enum-dispatch.md).

## Options

All options follow the trait path in `#[dispatch(..)]`.

- `Name = Type` fixes the associated type `Name` instead of taking it from
  the first variant. A method returning `Self::Name` is converted with
  `Into`, one returning `Result<_, Self::Name>` with `map_err(Into::into)`,
  so variants with their own error types can share one enum:

  ```rust
  #[whichever::dispatch(Storage, Error = StorageError)]
  pub enum AnyStorage {
      Memory(MemoryStorage), // Error = StorageError
      Disk(DiskStorage),     // Error = io::Error, which is Into<StorageError>
  }
  ```

- `Name = value` fixes the associated const `Name`, for a const that
  differs per payload by design, such as a kind discriminator:

  ```rust
  #[whichever::dispatch(Proof, KIND = "any")]
  pub enum AnyProof {
      Encoding(EncodingProof), // KIND = "encoding"
      Ordering(OrderingProof), // KIND = "ordering"
  }
  ```

- `mismatch(method = expr, ..)` gives the result of `method` when its
  `Self` arguments hold a different variant than `self`. The trait author
  can decide once instead, with `#[dispatch(mismatch = expr)]` on the
  method:

  ```rust
  #[whichever::dispatchable]
  pub trait Shape {
      /// Two different kinds of shape are never congruent.
      #[dispatch(mismatch = false)]
      fn congruent(&self, other: &Self) -> bool;
  }
  ```

- `from = false` suppresses the `From<Payload>` impls.
- `try_from = true` adds `TryFrom<Enum> for Payload` for every payload type,
  with the enum as the error, so a mismatch hands the value back.

Two options go on a variant. `#[dispatch(from = false)]` leaves the
variant out of the `From` and `TryFrom` impls, for a payload such as
`String` that should not convert into the enum. `#[dispatch(deref)]` calls
the trait on what the payload dereferences to, so `Box<T>`, `Rc<T>`,
`Arc<T>`, `&T` and `&mut T` payloads dispatch like a plain `T`:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape<'a> {
    #[dispatch(deref)]
    Boxed(Box<Circle>),
    #[dispatch(deref)]
    Borrowed(&'a Square),
    Plain(Square),
}
```

Several traits are dispatched by repeating the attribute; a supertrait is
one more attribute:

```rust
#[whichever::dispatch(Shape)]
#[whichever::dispatch(Drawable)]
pub enum AnyShape { /* .. */ }
```

A generic trait is dispatched at a concrete instantiation:
`#[whichever::dispatch(Encoder<Json>)]`. Type, lifetime and const parameters
are substituted in every signature, and `F::Value` becomes
`<Json as Format>::Value` when `F` has one bound.

## Inherent methods

An enum often needs methods of its own that only some variants can answer.
`#[dispatch]` on an inherent `impl` block forwards each marked method to the
named variants' method of the same name; the body is what every other
variant returns. `*` names every variant, and the method then has no body:

```rust
#[whichever::dispatch]
impl AnyShape {
    /// Only circles have a radius.
    #[dispatch(Circle, with = Some)]
    pub fn radius(&self) -> Option<f64> {
        None
    }

    #[dispatch(*)]
    pub fn describe(&self) -> String;

    #[dispatch(*, call = describe)]
    pub fn label(&self) -> String;
}
```

`with = path` wraps each forwarded result, `call = name` forwards to a
method of another name, `async` methods and methods returning
`impl Future` are awaited, and methods without the attribute are left
alone. A method returning `Self`, `Option<Self>` or `Result<Self, E>` wraps
the payload's result back into the variant, as trait dispatch does. With
`with`, the payload returns the plain value and `with` lifts the wrapped
variant: `#[dispatch(Circle, with = Some)]` on `fn grown(&self) ->
Option<Self>` calls a `Circle::grown` that returns a `Circle`. The `impl`
block has to be in the enum's crate, which inherent impls are anyway; a
bare `#[dispatch]` on an enum registers it for this without dispatching a
trait.

## Traits from `std` and other crates

`#[dispatch]` reads a trait's signatures from the hidden macro that
`#[dispatchable]` leaves next to the trait, so a trait from `std` or from a
crate you do not own cannot be dispatched as it is. Restate it once, with
`remote` naming the real trait, and dispatch the restatement:

```rust
#[whichever::dispatchable(remote = std::io::Write)]
trait Write {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    fn flush(&mut self) -> std::io::Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
}

#[whichever::dispatch(Write)] // implements std::io::Write
pub enum Stream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(UnixStream),
}
```

Nothing is emitted for the restatement but the macro, so the name is free
for `use std::io::Write;` in the same module. Exactly the methods restated
are forwarded, provided ones included; a changed signature fails to compile
against the real trait. The restatements an enum most often needs ship in
[`whichever::foreign`](https://docs.rs/whichever/latest/whichever/foreign/):
`Display`, `Debug`, `Error`, `PartialEq`, `Iterator`, `Read`, `Write`,
`BufRead` and `Seek`, each forwarding every provided method a `match` can
forward. `#[dispatch(whichever::foreign::Display)]` on an error enum
replaces the usual hand-written `match`, and the variant that holds a
`String` keeps `From` away with `#[dispatch(from = false)]`:

```rust
#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
#[whichever::dispatch(whichever::foreign::Error)]
pub enum ClientError {
    Io(io::Error),
    Parse(ParseIntError),
    #[dispatch(from = false)]
    BadUrl(String),
}
```

A method the enum answers differently from every payload, such as
`Error::source` returning `None` for some variants, goes in a
[partial impl block](#methods-the-enum-implements-itself).

## Methods the enum implements itself

A trait method without a receiver, such as a constructor, has no variant to
dispatch on, and sometimes the enum's answer to a method differs from every
payload's. Put `#[dispatch]` on a trait impl block holding just those
items, and the rest of the trait is forwarded around them:

```rust
#[whichever::dispatch]
pub enum Handler {
    Text(TextHandler),
    Map(MapHandler),
}

#[whichever::dispatch]
impl HandlerTrait for Handler {
    fn from_handler(handler: Handler) -> Option<Self> {
        Some(handler)
    }
}
```

The enum then does not name that trait in its own attribute, since the
block is the impl. Options for the trait, `Name = Type` and `mismatch`, go
on the block's attribute; an associated type written in the block counts as
an override too. A method written in the block replaces the forwarding for
every variant, provided methods included, which is the way to leave a
payload's method out on purpose. The block's own header, generics and
bounds included, becomes the impl's header. Like inherent blocks, it has to
be in the enum's crate.

## What each trait item becomes

| Trait item | Enum impl |
| --- | --- |
| `type X;` | `type X = <FirstVariant as Trait>::X;`, or the `X = Type` option. Every other variant is checked to agree; a disagreement is reported on that variant. |
| `const N: T;` | `const N: T = <FirstVariant as Trait>::N;`, or the `N = value` option. For an integer, `bool`, `char` or `&str` const on a non-generic enum, every other variant is checked to agree at compile time, with the error on that variant. |
| method with a receiver | A `match` over the variants calling `<Payload as Trait>::method(payload, args..)`, with `#[inline]`. |
| default body | Ignored: forwarded like any other method. |
| `async fn`, `unsafe fn` | Arms are awaited; calls are wrapped in `unsafe { }`. |
| `-> impl Future<Output = T>` | The `match` is written in one `async move` block and every arm is awaited, as for `async fn`; `Send` and any other bound are checked against that block. |
| argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`; different variants give the `mismatch` expression. |
| return `Self` | Wrapped back into the same variant; `Option<Self>` and `Result<Self, E>` are mapped, under any path, `io::Result<Self>` included. `Self` anywhere else in a return type is an error. |
| method without a receiver | Left to the trait's default body, or written in a `#[dispatch] impl Trait for Enum` block; an error when it has neither. |
| `#[cfg]` on a method or variant | Carried onto the generated method or arm. |

## Limits

- Every variant holds exactly one unnamed field, `Name(Payload)`, and
  every payload implements the trait.
- Return-position `impl Trait` cannot be dispatched by a `match`, except
  `impl Future<Output = T>`, which is awaited like an `async fn`.
- A `deref` variant cannot take part in a method returning `Self`: there is
  no way to put the result back behind the pointer.
- Generic associated types and `self: Type` receivers are not supported.
- The types in the trait's signatures are written into the enum's module
  and must be in scope there, as for a hand-written impl; `crate::` paths
  are rewritten to the trait's crate, `super::` and `self::` paths are not.
  They must also be nameable from there: a private alias or type in a
  signature limits the trait to its own crate.
- An associated const of a type other than an integer, `bool`, `char` or
  `&str`, or on a generic enum, is taken from the first variant without a
  check.
- One `macro_rules!` may emit a `#[dispatchable]` trait in several modules
  when at least one token of the trait, its name or a method's, comes from
  the invocation. A trait written out in full in the macro's body gets the
  same hidden macro name at every invocation, and rustc reports it as
  defined multiple times, unless each invocation passes its own
  `#[dispatchable(name = ..)]`.

Every unsupported shape is reported as one sentence on the offending trait
item or variant; see `tests/ui` for all of them. A rejected enum is still
emitted, so the derives, uses and impl blocks around it compile and that
one error stands alone.

"cannot find macro `Shape` in this scope" at the enum means the trait is not
marked `#[dispatchable]`, or the trait, which carries the macro, is not
imported where the enum is. A `std::`, `core::` or `alloc::` path in
`#[dispatch]` is reported directly, with the `whichever::foreign`
declaration or the `remote` restatement to use instead.

One `macro_rules!` that emits the same `pub` trait, written out in full,
at two places gets the same hidden macro name twice; `#[dispatchable(name =
tokio)]` names the macro explicitly and tells the two apart.

## Compatibility

`no_std`, with no dependencies beyond the proc-macro crate. The minimum
supported Rust version is 1.88. Both the crate that owns the trait and the
crate that owns the enum depend on `whichever`; renaming the dependency is
fine.

`#[dispatchable]` on a `pub` trait adds one `#[doc(hidden)]` exported macro
to the crate root, named after the trait and the position of its tokens,
which is how another crate's enum reaches the trait's signatures. A trait
of any other visibility keeps its macro in its own module and adds nothing
to the crate root.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
