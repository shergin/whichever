# whichever

[![crates.io](https://img.shields.io/crates/v/whichever.svg)](https://crates.io/crates/whichever)
[![docs.rs](https://docs.rs/whichever/badge.svg)](https://docs.rs/whichever)
[![CI](https://github.com/shergin/whichever/actions/workflows/ci.yml/badge.svg)](https://github.com/shergin/whichever/actions/workflows/ci.yml)

Implements a trait for an enum by forwarding to whichever variant it holds.

An enum over a trait's implementors is the cheapest way to hold "one of
these": no `Box`, no vtable, no allocation, and every arm visible to the
optimiser. The price is the forwarding impl, a `match` per method, paid
again for every method and every variant. `whichever` writes that impl,
the one you would write by hand, from one attribute on the trait and one
on the enum.

It keeps working where `enum_dispatch` stops: the trait in one crate and
the enum in another, associated types and consts, `&Self` arguments,
`Option<Self>` and `Result<Self, E>` returns, traits you do not own. The
crates that followed each cover some of that; `whichever` covers all of
it, and what it generates is the hand-written `match`, instruction for
instruction.

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

`AnyShape` now implements `Shape`:

```rust
impl Shape for AnyShape {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            AnyShape::Circle(inner) => <Circle as Shape>::area(inner),
            AnyShape::Square(inner) => <Square as Shape>::area(inner),
        }
    }
    // `name` is forwarded the same way, and `From<Circle>` and
    // `From<Square>` come with the impl.
}
```

This is the code a careful hand writes: one `match` per method, a fully
qualified call per arm, `#[inline]`; no trait object, no closure, no
helper trait, no allocation. [What is generated](docs/what-is-generated.md)
has the full account.

Run `cargo add whichever` in the crate that owns the trait and in the
crate that owns the enum.

## Where the alternatives stop

Several crates write this forwarding impl. Four of them, and where each
stops:

- **`enum_dispatch`** came first, and `whichever` keeps its two
  attributes. It remembers the trait in a cache inside the proc-macro
  server, so an enum in another crate gets no impl, silently, and
  associated types and consts, `&Self` arguments and traits you do not
  own are out of reach.
- **`ambassador`** works across crates and handles associated types and
  generic traits, but no `Self` in arguments or returns, no associated
  consts and no `From` impls.
- **`enum_delegate`** handles `-> Self` and unifies associated types, but
  panics at runtime on `&Self` arguments and has no `async`, no generic
  traits and no generic methods.
- **`delegate`** handles almost anything, because you list every
  signature again by hand.

`whichever` keeps nothing between expansions: the trait's signatures
travel in a small macro left next to the trait, the carrier macro, so the
trait reaches the enum through ordinary name resolution, across crates,
in any order and under rust-analyzer. And it forwards every shape above.
[The comparison](docs/comparison.md) has the full table against seven
crates, the requests still open on `enum_dispatch`'s tracker, and a trait
it cannot take, dispatched with the same two attributes.

## What it forwards

- **[Associated types and consts](docs/options.md)**, taken from the
  first variant and checked to agree on the others, or fixed on the enum.
- **[`Self` in signatures](docs/what-is-generated.md)**: `&Self` and
  `Self` arguments with a declared fallback for mismatched variants, and
  `-> Self`, `-> Option<Self>`, `-> Result<Self, E>` wrapped back into the
  variant.
- **[Traits you do not own](docs/restatements.md)**: `Display`, `Debug`,
  `Error`, `PartialEq`, `Iterator`, `DoubleEndedIterator`,
  `ExactSizeIterator`, `Read`, `Write`, `BufRead` and `Seek` ship in
  `whichever::foreign`; any other trait is restated once, and so is an
  enum another crate defines.
- **[Some methods written by hand](docs/partial-impl-blocks.md)**, the
  rest forwarded: constructors, methods without a receiver, or a method
  the enum answers differently from every payload.
- **[Generic traits](docs/options.md)** at a concrete instantiation, with
  type, lifetime and const parameters.
- **[`Box<T>`, `Rc<T>` and `&T` payloads](docs/routes.md)**, dispatched
  like a plain `T`, or a field of the payload as the target.
- **[`async fn`, `impl Future`, `self: Pin<&mut Self>` and
  `self: Box<Self>`](docs/what-is-generated.md)**.
- **[Inherent methods](docs/inherent-methods.md)** forwarded to some
  variants, with a fallback for the rest.
- **[A struct forwarding to one field](docs/structs.md)**, for the public
  wrapper around a private dispatch enum.

A shape it cannot forward is one sentence on the trait item or variant at
fault, with the way around it where one exists, and the rest of the enum
still compiles. [The limits](docs/limits.md) are listed, each with its
`tests/ui` case.

## Performance

`whichever` costs nothing over the hand-written `match`: the functions it
generates are identical to the hand-written ones, instruction for
instruction, and CI compares the assembly. [The
benchmarks](benchmarks/README.md) set it against a hand-written impl,
`Box<dyn Trait>` and the enums of `enum_dispatch`, `ambassador`,
`enum_delegate`, `delegation` and `delegate`, called from another crate
without LTO, over 2 to 32 variants in uniform, cyclic and random layouts.
From the last run, on an Apple M1 Pro
([full tables](benchmarks/RESULTS.md)):

- Over a random 8-variant collection, one `&self` call costs 1.7 ns per
  element against 6.8 ns through a `Box<dyn Trait>` when the method
  bodies can be inlined, and 8.4 ns against 6.7 ns when they cannot. The
  enum's advantage is what the optimiser gets to see, not the dispatch.
- Where the payload's place matters, the gap is wide: 1.7 ns against
  6.5 ns per element over 4 Mi scattered elements, and a collection is
  built 13x faster without a box per element.
- A generated impl without `#[inline]` (`enum_delegate`, `delegation`)
  pays an extra out-of-line call across the crate boundary, up to 5x
  depending on the method, the variant count and the layout.

Whether an enum is the right choice for a given program is answered in
[When enum dispatch pays](docs/when-to-use-enum-dispatch.md).

## Moving from `enum_dispatch`

The swap is mechanical:

| `enum_dispatch` | `whichever` |
| --- | --- |
| `#[enum_dispatch]` on the trait | `#[whichever::dispatchable]` |
| `#[enum_dispatch(Trait)]` on the enum | `#[whichever::dispatch(Trait)]` |
| `enum E { Circle, Square }` | `enum E { Circle(Circle), Square(Square) }` |

The rest of the mapping, and the three things to watch, are in
[docs/moving-from-enum-dispatch.md](docs/moving-from-enum-dispatch.md).

## Checked, not claimed

Every capability has a behaviour test, and every limit a compile-fail
test with its wording pinned; the generated impls are pinned by expansion
goldens. CI builds the trait and the enum in separate crates, builds for
a `no_std` target, runs on stable, beta and the minimum supported Rust,
and compares the generated assembly with the hand-written impl. A shape
enters the crate only when a surveyed project writes that forwarding by
hand ([rule 3](docs/vision.md#the-rules)).

The footprint: `no_std`, and the generated code names only `core`. Rust
1.88 or later. With `default-features = false` the macro crate's whole
dependency tree is `proc-macro2`, `quote` and `syn` 3; the default feature
`renamed-dependency` adds `proc-macro-crate`, for a crate that renames
its `whichever` dependency. Before 1.0 the trait's crate and the enum's
crate use the same minor version; [Compatibility](docs/compatibility.md)
has the details.

## Documentation

- [docs/README.md](docs/README.md): every page, by the question it
  answers.
- The API reference on [docs.rs](https://docs.rs/whichever), with
  runnable examples.
- [When enum dispatch pays](docs/when-to-use-enum-dispatch.md): enum,
  trait object or generic, with the measurements.
- The [vision](docs/vision.md) and the [decisions](docs/decisions/): why
  it is shaped this way.
- The [benchmarks](benchmarks/README.md) and their
  [results](benchmarks/RESULTS.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
