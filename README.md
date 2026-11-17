# whichever

[![crates.io](https://img.shields.io/crates/v/whichever.svg)](https://crates.io/crates/whichever)
[![docs.rs](https://docs.rs/whichever/badge.svg)](https://docs.rs/whichever)
[![CI](https://github.com/shergin/whichever/actions/workflows/ci.yml/badge.svg)](https://github.com/shergin/whichever/actions/workflows/ci.yml)

Implements a trait for an enum by forwarding to whichever variant it holds.

You have a trait and a few types that implement it, and you want to hold
"one of these". An enum is the cheapest way to do that: no `Box`, no
vtable, no allocation, and the optimiser can see into every arm.

The catch is the boilerplate. The enum needs its own impl of the trait,
and every method in it is the same `match`, passing the call on to
whichever variant is inside. Ten methods and six variants is sixty arms
of nothing.

`whichever` writes that impl for you. Put one attribute on the trait and
one on the enum, and you get the code you would have written by hand.

## What it looks like

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

`AnyShape` now implements `Shape`. This is the impl you get (with shorter
names for the bindings):

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

Notice what is not there: no trait object, no closure, no helper trait,
no allocation. It is one `match` per method, a fully qualified call per
arm, and `#[inline]`, which is what you would write by hand.
[What is generated](docs/what-is-generated.md) shows the whole expansion.

## Install

Run `cargo add whichever` in the crate that owns the trait and in the
crate that owns the enum.

New to the crate? [The tutorial](docs/tutorial.md) is a guided tour,
about fifteen minutes.

## What it forwards

- **[Associated types and consts](docs/options.md).** Taken from the
  first variant and checked to agree on the others, or fixed on the enum.
- **[`Self` in signatures](docs/what-is-generated.md).** `&Self` and
  `Self` arguments, with a declared mismatch value (what to return when
  `self` and the argument hold different variants); `-> Self`,
  `-> Option<Self>` and `-> Result<Self, E>` wrapped back into the
  variant.
- **[Traits you do not own](docs/restatements.md).** `Display`, `Debug`,
  `Error`, `PartialEq`, `Iterator`, `DoubleEndedIterator`,
  `ExactSizeIterator`, `Read`, `Write`, `BufRead` and `Seek` ship in
  `whichever::foreign`. You restate any other trait once, and the same
  goes for an enum another crate defines.
- **[Some methods written by hand](docs/partial-impl-blocks.md), the rest
  forwarded.** Constructors, methods without a receiver, a method the
  enum handles differently from every payload, or the arms of the few
  variants it handles itself.
- **[Generic traits](docs/options.md).** At a concrete instantiation,
  with type, lifetime and const parameters.
- **[`Box<T>`, `Rc<T>` and `&T` payloads](docs/routes.md).** Treated
  like a plain `T`, including a `Box<dyn Trait>` for an open set, or with
  a field of the payload as the target.
- **[`async fn`, `impl Future`, `self: Pin<&mut Self>` and
  `self: Box<Self>`](docs/what-is-generated.md).**
- **[Inherent methods](docs/inherent-methods.md).** Forwarded to some
  variants, with a fallback for the rest.
- **[A struct forwarding to one field](docs/structs.md).** For the public
  wrapper around a private
  [dispatch enum](docs/glossary.md#dispatch-enum).

When a trait method or variant cannot be forwarded, you get a
one-sentence error on the item at fault, with the fix where there is one.
The rest of the enum still compiles, so you see the error for the thing
to fix, and no wall of follow-on errors. [The limits](docs/limits.md) are
listed, each with its `tests/ui` case.

## Inline for the common kinds, `dyn` for the rest

An enum is a closed set, and that is what makes it fast. Sometimes one
more kind turns up that nobody listed: a plugin's type, a test double,
the rare case not worth a variant. Keep the enum, and give it one variant
that holds a trait object:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
    #[dispatch(deref)]
    Other(Box<dyn Shape>),
}
```

`Circle` and `Square` stay inline: no allocation, and a `match` arm the
optimiser can inline. `Other` takes any implementor of `Shape`, from any
crate, and pays the vtable call a trait object always pays. The common
kinds keep the enum's speed, and the set stays open.

The trait has to be dyn compatible (what Rust used to call object safe)
for that variant. A method that needs `Self: Sized` cannot be called on a
trait object: you get an error for it, and the fix is one line in a
[partial impl block](docs/glossary.md#partial-impl-block).
[Reaching the target](docs/routes.md) has the details, and
[When enum dispatch pays](docs/when-to-use-enum-dispatch.md#patterns)
sets this pattern beside its neighbours.

## Where the alternatives stop

Several crates write this forwarding impl. Here are four, and where each
one stops:

- **`enum_dispatch`** came first, and `whichever` keeps its two
  attributes. It remembers the trait in a cache inside the proc-macro
  server. So an enum in another crate silently gets no impl, and
  associated types and consts, `&Self` arguments and traits you do not
  own are out of reach.
- **`ambassador`** works across crates and handles associated types and
  generic traits. It has no `Self` in arguments or returns, no
  associated consts and no `From` impls.
- **`enum_delegate`** handles `-> Self` and unifies associated types. It
  panics at runtime on `&Self` arguments, and has no `async`, no generic
  traits and no generic methods.
- **`delegate`** handles almost anything, because you list every
  signature again by hand.

`whichever` keeps nothing between expansions. Instead, `#[dispatchable]`
leaves a small hidden macro next to the trait that holds the trait's
signatures: the [carrier macro](docs/glossary.md#carrier-macro). The enum
reaches the trait through ordinary name resolution, so it works across
crates, in any order, and under rust-analyzer.

[The comparison](docs/comparison.md) has the full table against seven
crates, the requests still open on `enum_dispatch`'s tracker, and a trait
`enum_dispatch` cannot take, dispatched with the same two attributes.

The crates that came after `enum_dispatch` each cover some of what it
cannot do. `whichever` covers all of it.

## Performance

`whichever` costs nothing over the hand-written `match`. The functions it
generates are identical to the hand-written ones, instruction for
instruction, and CI compares the assembly.

[The benchmarks](benchmarks/README.md) set it against a hand-written
impl, `Box<dyn Trait>` and the enums of `enum_dispatch`, `ambassador`,
`enum_delegate`, `delegation` and `delegate`. Each is called from another
crate without LTO, over 2 to 32 variants in uniform, cyclic and random
layouts. From the last run, on an Apple M1 Pro
([full tables](benchmarks/RESULTS.md)):

- **Over a random 8-variant collection**, one `&self` call costs 1.7 ns
  per element against 6.7 ns through a `Box<dyn Trait>` when the method
  bodies can be inlined, and 8.5 ns against 6.6 ns when they cannot. The
  enum's advantage is what the optimiser gets to see, not the dispatch.
- **Where the payload's place in memory matters**, the gap is wide:
  1.7 ns against 5.5 ns per element over 4 Mi (about four million)
  scattered elements. And a
  collection is built 13x faster without a box per element.
- **A generated impl without `#[inline]`** pays an extra out-of-line
  call across the crate boundary: up to 3x for `enum_delegate`, and more
  than 5x for `delegation`, which also walks a chain of `Either`s instead
  of a `match`. How much depends on the method, the variant count and the
  layout.

Is an enum the right choice for your program at all?
[When enum dispatch pays](docs/when-to-use-enum-dispatch.md) answers that.

## Moving from `enum_dispatch`

The swap is mechanical:

| `enum_dispatch` | `whichever` |
| --- | --- |
| `#[enum_dispatch]` on the trait | `#[whichever::dispatchable]` |
| `#[enum_dispatch(Trait)]` on the enum | `#[whichever::dispatch(Trait)]` |
| `enum E { Circle, Square }` | `enum E { Circle(Circle), Square(Square) }` |

[Moving from `enum_dispatch`](docs/moving-from-enum-dispatch.md) walks
through it step by step, with the rest of the mapping and the three
things to watch.

## Checked, not claimed

- Every capability has a behaviour test.
- Every limit has a compile-fail test with its wording pinned.
- Expansion goldens pin the generated impls.
- CI builds the trait and the enum in separate crates, builds for a
  `no_std` target, runs on stable, beta and the minimum supported Rust,
  and compares the generated assembly with the hand-written impl.
- A new kind of trait method or variant is supported only when a
  surveyed project writes that forwarding by hand
  ([rule 3](docs/vision.md#the-rules)).

The footprint is small:

- `no_std`, and the generated code names only `core`.
- Rust 1.88 or later.
- With `default-features = false`, the macro crate's whole dependency
  tree is `proc-macro2`, `quote` and `syn` 3. The default feature
  `renamed-dependency` adds `proc-macro-crate`, for a crate that renames
  its `whichever` dependency.
- Before 1.0, the trait's crate and the enum's crate use the same minor
  version. [Compatibility](docs/compatibility.md) has the details.

## Documentation

- [The tutorial](docs/tutorial.md): a guided tour, start to finish.
- [docs/README.md](docs/README.md): every page, by the question it
  answers.
- [The glossary](docs/glossary.md): the words these pages use.
- [Troubleshooting](docs/troubleshooting.md): an error message, its
  cause and the fix.
- The API reference on [docs.rs](https://docs.rs/whichever), with
  runnable examples. The tutorial, the glossary and troubleshooting are
  there too, at the version you depend on.
- [When enum dispatch pays](docs/when-to-use-enum-dispatch.md): enum,
  trait object or generic, with the measurements.
- The [vision](docs/vision.md) and the [decisions](docs/decisions/): why
  it is shaped this way.
- The [benchmarks](benchmarks/README.md) and their
  [results](benchmarks/RESULTS.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
