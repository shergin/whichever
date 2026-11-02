# When enum dispatch pays

A program that holds "one of these types, all implementing a trait" has
three ways to do it: a generic parameter, a trait object, or an enum with one
variant per type. `whichever` makes the third as cheap to write as the
second. This page is about when it is the right one, with the measurements
from [`benchmarks/`](../benchmarks/README.md) behind each claim. The numbers
are from the run recorded in [`RESULTS.md`](../benchmarks/RESULTS.md), on an
Apple M1 Pro without link-time optimisation; another machine moves them, the
shapes hold.

## Three ways to hold one of several types

| | Generic `T: Trait` | `Box<dyn Trait>` | Enum over implementors |
| --- | --- | --- | --- |
| Set of types | fixed per call site | open: anyone may add one | closed: listed where the enum is |
| Where the value lives | inline | behind a pointer, on the heap | inline, in the enum |
| Size | the type's own | 16 bytes plus the heap object | the largest variant plus a tag |
| Method call | direct, inlinable | through a vtable, never inlined | a `match`, inlinable |
| What the trait may contain | anything | object-safe items only: no `-> Self`, no `Self` arguments, no generic methods, no associated consts | anything |
| Derives on the holder | the type's own | none | `Clone`, `Copy`, `PartialEq`, `Hash`, `serde`, whatever the payloads allow |
| Adding a type | nothing to change | nothing to change | one variant, and every exhaustive `match` on the enum |
| Code generated | one copy per instantiation | one | one impl per trait |

A generic is the floor: when the type is known where the call happens, there
is nothing to decide at runtime, and in the benchmarks a collection of one
type sums a method in 0.20 ns per element where any dispatch takes at least
0.40 ns. The choice this page is about is the one between a trait object and
an enum, for a value whose type is only known at runtime.

## What the measurements say

### The dispatch itself is not the win

Take the method's body out of the picture, by making every payload method a
real call that cannot be inlined, and an enum and a trait object cost nearly
the same per call. Over 1024 elements of 8 random variants, one `&self` call
costs 2.10 ns per element through the `match` and 2.34 ns through the
vtable. At 16 and 32 variants the compiler turns the `match` into a jump
table, which is an indirect branch like a vtable call, and when that branch
is predictable the jump table is the slower of the two: 1.27 ns against
0.96 ns over a uniform collection. The `match` wins clearly only where its
branch is cheap and the vtable's is not: 1.25 ns against 4.22 ns over two
variants in random order, where an indirect branch is wrong half the time.

So "enum dispatch is faster than dynamic dispatch" is, for the dispatch
alone, mostly false. Two things that come with the enum are not.

### Inlining is

With payload methods the optimiser is allowed to inline, the `match` is no
longer a call site but a piece of straight-line code the surrounding loop is
compiled with, and the vtable call is still a call. The same 8-variant random
collection: 1.69 ns against 2.30 ns for a method returning a value, 0.32 ns
against 2.32 ns for a `&mut self` method whose body is one multiply-add, and
0.22 ns against 1.85 ns for a method the trait gives a default body. Over
two random variants, where the `match` compiles to a branch-free select, it
is 0.40 ns against 4.37 ns.

This is where `enum_dispatch`'s "up to 10x" comes from, and it is real, for
methods small enough to inline. A method that does real work gains little
from inlining, and for it the previous section applies.

### Memory is the other win

An enum carries its payload with it; a `Box<dyn Trait>` points at a heap
object. While a collection is small and its boxes were allocated in order,
that costs little: 1.71 ns against 2.30 ns per element over 1 Ki elements.
Scatter the collection, as a program that has been running for a while
does, and the box is a cache miss per element: 1.68 ns against 6.64 ns at
16 Ki elements, 1.73 ns against 8.08 ns at 4 Mi. Building the collection is
the same story from the other side, 2.2 ns against 29.8 ns per element,
because the enum allocates nothing.

### Layout and the branch predictor

Both kinds of dispatch depend on the branch predictor, and the matrix in
`RESULTS.md` is laid out to show how much. In a uniform collection every
element takes the same branch and a vtable call costs 0.96 ns at any variant
count. In a cyclic one, the predictor learns the period and the `match` keeps
up with the uniform case while the vtable call does not (1.74 to 2.55 ns).
In a random one both pay, and the gap between them is what the previous two
sections describe. If a collection can be grouped by variant before a hot
loop, that costs nothing and helps both.

### The crate boundary

A trait method on an enum generated in one crate and called from another is
inlined into the caller only when the generated method is generic, carries
`#[inline]`, or is small and free of calls, and a `match` of calls is never
the last. A strategy that omits `#[inline]` therefore leaves every call out
of line without LTO: `enum_delegate` pays 15% to 2x for it in the
benchmarks, and `delegation`, which also walks a chain of `Either`s instead
of a `match`, up to 4x. `whichever` emits `#[inline]` on every
generated method, and the generated-code table in `RESULTS.md` shows its
functions identical to the hand-written ones, so the numbers above are its
numbers. Fat LTO closes most of that gap for the others and is a fair thing
to turn on in a release build anyway.

## When an enum pays

- The set of types is closed, and known where the enum is defined. A
  parser's node kinds, an interpreter's values, the shapes a renderer draws,
  the backends a service can be configured with, the messages of a protocol.
- A hot loop visits many such values, and their methods are small. The
  smaller the method, the larger the gain, up to an order of magnitude.
- The values live in a collection. Inline storage saves an allocation per
  element on the way in and a cache miss per element on every pass.
- The trait needs what a trait object cannot offer: `-> Self`, a `&Self`
  argument (`fn congruent(&self, other: &Self)`), associated types or
  consts, generic methods, or the holder needs `Clone`, `PartialEq`,
  `Hash` or serialisation. An enum has all of them; `whichever` forwards
  them, with a declared fallback for `Self` arguments that hold a different
  variant.
- The code has to run without an allocator, or with every value on the
  stack, as in `no_std` targets.
- Exhaustiveness is wanted: a new kind of value should break every place
  that forgot to handle it.

## When it does not

- The set is open. Types provided by other crates, plugins, or anything a
  user of the library can add cannot be listed in an enum. A trait object is
  the right tool, or see the escape hatch below.
- The variants differ a lot in size. An enum is as large as its largest
  variant, so one 200-byte variant next to 16-byte ones makes every element
  200 bytes and a `Vec` of them mostly padding, which costs more in memory
  traffic than the dispatch saves. Box the large variants (below).
- The method bodies are large and the calls infrequent. The dispatch then
  costs the same either way, inlining gains nothing, and the trait object's
  flexibility costs nothing.
- A type is known at the call site after all. A generic is cheaper than
  either, and the enum's `From` impls make it easy to stay generic until
  the point where a value really has to be one of several.
- Code size matters and the trait is wide. Each method of the enum is a
  `match` over every variant, and with inlinable bodies each arm is a copy
  of the body; a vtable is one pointer per method.
- Expecting the 10x. It is there for tiny methods over heterogeneous
  collections and nowhere else; for most code the gain is between 10% and
  2x, and memory decides more than dispatch.

## Patterns

**An escape hatch.** A closed set with one variant for everything else keeps
the common kinds inline and fast and still admits a type nobody listed.
`whichever` dispatches a `Box<dyn Trait>` payload through
`#[dispatch(deref)]`, as long as the trait is object safe:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Square(Square),
    Circle(Circle),
    #[dispatch(deref)]
    Other(Box<dyn Shape>),
}
```

**Boxing the outliers.** The same attribute keeps a large or rare variant
behind a pointer, so the enum stays the size of its common variants:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Square(Square),
    Circle(Circle),
    #[dispatch(deref)]
    Polygon(Box<Polygon>), // hundreds of bytes, seldom seen
}
```

A `deref` variant cannot take part in a method returning `Self`, since there
is no way to put the result back behind the pointer; everything else
dispatches as for a plain payload.

**Grouping before a hot loop.** Sorting or bucketing a collection by variant
turns a random layout into a uniform one and is worth more than any choice
of dispatch when the loop runs often enough to amortise it.

**Generic inside, enum outside.** Keep the kernel generic over `T: Trait`
and let the enum choose which instantiation to call once per batch rather
than once per element. The dispatch then costs nothing that matters, and
each instantiation is compiled at the floor.

## Getting the full benefit

- Hold the enum in a `Vec<Enum>`, not a `Vec<Box<Enum>>`; the point is the
  inline payload.
- Payload methods that should inline across a crate boundary need
  `#[inline]` too, or LTO; rustc inlines a small call-free function from
  another crate on its own, but not one that calls anything.
- Keep the variants close in size, boxing the ones that are not.
- Group collections by variant where the program allows it.
- Measure on the machine that matters. The suite runs with
  `cargo bench -p whichever-benchmarks` and writes its tables with
  `benchmarks/scripts/report.py`; adding a subject that mirrors a real
  trait is a few lines, see the benchmark README.

## Why `whichever` and not a hand-written impl

The generated impl is the hand-written one, instruction for instruction,
as `benchmarks/scripts/codegen.py` checks against every subject. What the
attribute adds is that the impl stays correct when the trait grows, forwards
the items a hand gets wrong or leaves out (default methods, `Self`
arguments, associated types with differing concrete types), and works with a
trait from another crate. The comparison with the other crates that do this
is the table in the [README](../README.md#why-another-one).
