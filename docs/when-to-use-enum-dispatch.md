# When enum dispatch pays

You have a value that is "one of these types, all implementing a trait".
Should you hold it in a generic, a trait object or an enum? And what does
each one cost?

The short version:

- **An enum pays** when the set of types is closed, the methods are small,
  and the values live in a collection that a hot loop walks.
- **It does not pay** when the set is open or the variants differ a lot
  in size. And when the methods do real work, a trait object costs about
  the same.
- The rest of this page shows the measurements behind both answers.

`whichever` makes the enum as cheap to write as the trait object. The
numbers come from the benchmark suite in
[`benchmarks/`](../benchmarks/README.md), as recorded in
[`RESULTS.md`](../benchmarks/RESULTS.md). They were measured on an Apple M1
Pro without link-time optimisation (LTO, where the linker optimises across
crates as if they were one). Another machine moves the numbers, but the
pattern stays the same.

## Three ways to hold one of several types

| | Generic `T: Trait` | `Box<dyn Trait>` | Enum over implementors |
| --- | --- | --- | --- |
| Set of types | fixed per call site | open: anyone may add one | closed: listed where the enum is |
| Where the value lives | inline | behind a pointer, on the heap | inline, in the enum |
| Size | the type's own | 16 bytes plus the heap object | the largest variant plus a tag |
| Method call | direct, inlinable | through a vtable, never inlined | a `match`, inlinable |
| What the trait may contain | anything | dyn compatible items only (what Rust used to call object safe): no `-> Self`, no `Self` arguments, no generic methods, no associated consts | anything |
| Derives on the holder | the type's own | none | `Clone`, `Copy`, `PartialEq`, `Hash`, `serde`, whatever the payloads allow |
| Adding a type | nothing to change | nothing to change | one variant, and every exhaustive `match` on the enum |
| Code generated | one copy per instantiation | one | one impl per trait |

Two terms from the table:

- A **vtable** is the table of function pointers behind a trait object. A
  call through `Box<dyn Trait>` loads the right pointer from it and calls
  it.
- **Inlining** is when the compiler copies a function's body into the
  place that calls it. The call disappears, and the body can be optimised
  together with the code around it.

A generic is the floor. When the type is known where the call happens,
there is nothing to decide at runtime. In the benchmarks, a collection of
one type sums a method in 0.20 ns per element, where any dispatch takes at
least 0.40 ns.

So the real choice on this page is between a trait object and an enum, for
a value whose type is only known at runtime.

## What the measurements say

A few words used below:

- The **branch predictor** is the part of the CPU that guesses which way a
  branch goes before it knows. A right guess is nearly free. A wrong guess
  throws away the work done on the wrong path.
- A **jump table** is how the compiler often builds a large `match`: an
  array of code addresses, indexed by the variant. Like a vtable call, it
  is one indirect branch.
- **Ki** and **Mi** count elements in binary thousands and binary
  millions (powers of 1024).
- A collection's **layout** is the order of its variants. In a **uniform**
  collection every element holds the same variant. In a **cyclic** one the
  variants repeat in a fixed order. In a **random** one each element's
  variant is drawn at random.

### The dispatch itself is not the win

**For the dispatch alone, an enum is no faster than a trait object.**

To see this, take the method's body out of the picture: make every payload
method a real call that cannot be inlined. Then the only difference left is
how each side picks the function to call.

Up to 8 variants, the `match` is a chain of compares. A random variant
can send the predictor the wrong way more than once per element. The
vtable call is one indirect branch. At 16 and 32 variants the compiler
turns the `match` into a jump table, and the two cost nearly the same over
a random collection. Over a uniform one, where every branch is predicted,
the jump table is the slower.

One `&self` call per element, over 64 Ki elements:

| Case | Enum (`match`) | `Box<dyn Trait>` (vtable) |
| --- | --- | --- |
| 2 variants, random | 3.70 ns | 4.71 ns |
| 8 variants, random | 8.50 ns | 6.62 ns |
| 16 variants, random (jump table) | 7.74 ns | 6.92 ns |
| 32 variants, uniform (jump table) | 1.25 ns | 0.95 ns |

The `match` wins only in the first row: two variants in random order.

So "enum dispatch is faster than dynamic dispatch" is false for the
dispatch alone. But two things that come with the enum are real wins.

### Inlining is the first win

**When the payload methods are small enough to inline, an enum is several
times faster over a handful of variants.**

Once the optimiser may inline the payload methods, the `match` stops being
a call. It becomes straight-line code that the compiler builds together
with the surrounding loop. The vtable call is still a call.

With up to 8 small arms, the compiler goes further: it computes every arm
and keeps the one it needs. No branch is left, so there is nothing for the
predictor to get wrong.

Random collections:

| Case | Enum (`match`) | `Box<dyn Trait>` (vtable) |
| --- | --- | --- |
| 8 variants, method returning a value | 1.66 ns | 6.60 ns |
| 8 variants, `&mut self` method whose body is one multiply-add | 0.37 ns | 6.58 ns |
| 8 variants, method the trait gives a default body | 0.33 ns | 5.10 ns |
| 2 variants | 0.41 ns | 4.62 ns |
| 16 variants (jump table) | 7.16 ns | 7.20 ns |

This is where `enum_dispatch`'s "up to 10x" comes from. It is real, for
methods small enough to inline, over a handful of variants.

At 16 and 32 variants the gain is gone, as the last row shows. The compiler
builds a jump table instead, and a random collection sends it the wrong way
as often as it does a vtable call. And a method that does real work gains
little from inlining; for it, the previous section applies.

### Memory is the second win

**An enum carries its payload with it, so it never chases a pointer. Once
the collection outgrows the cache, that matters on every element.**

A `Box<dyn Trait>` points at an object on the heap. While the heap objects
fit in the cache, where they sit costs nothing. Past the cache, every
element is a trip to memory.

The benchmark builds a collection of one variant and then scatters it, as
in a program that has been running for a while:

- A call through the box costs 0.94 ns per element at 1 Ki elements and
  0.95 ns at 256 Ki.
- At 4 Mi elements it costs 5.54 ns. The enum, walked in order, costs
  1.71 ns, as it does at every size.

Building the collection is the same story from the other side: 2.15 ns per
element for the enum against 28.84 ns for the boxes, because the enum
allocates nothing.

### Layout and the branch predictor

**How the variants are ordered in a collection matters as much as the
choice of dispatch, and often more.**

Both kinds of dispatch depend on the branch predictor. The matrix in
`RESULTS.md` is laid out to show how much:

- **Uniform:** every element takes the same branch. A vtable call costs
  0.94 to 0.96 ns at any variant count.
- **Cyclic:** the predictor learns the period. Up to 8 variants the
  `match` keeps up with the uniform case, and at 16 and 32, where it is a
  jump table, it costs 1.52 to 1.56 ns. The vtable call does not keep up
  at any count (1.81 to 2.58 ns).
- **Random:** both pay. The gap between them is what the previous two
  sections describe. From 16 variants, where the `match` is a jump table,
  they pay the same.

"Random" means random to the predictor. A loop that replays the same
thousand random variants teaches it the sequence. That is why the suite
measures over 64 Ki elements.

A collection grouped by variant is a uniform one for most of its length.
That helps both kinds of dispatch by more than the choice between them. At
32 variants:

| Layout | Enum (`match`) | `Box<dyn Trait>` (vtable) |
| --- | --- | --- |
| Random | 7.32 ns | 7.43 ns |
| Uniform, which is what grouping gives you | 0.64 ns | 0.96 ns |

### The crate boundary

**Without LTO, a generated `match` of calls is inlined into a caller in
another crate only when it is generic or carries `#[inline]`. `whichever`
always writes `#[inline]`.**

A trait method on an enum generated in one crate and called from another
is inlined into the caller only when the generated method:

- is generic,
- carries `#[inline]`, or
- is small and free of calls.

A `match` whose arms are calls is never call-free, so it is the
`#[inline]` that lets it cross the crate boundary. So a crate that leaves
out `#[inline]` leaves every call out of line without LTO. In the benchmarks,
`enum_delegate` pays up to 3x for it. `delegation`, which also walks a
chain of `Either`s instead of a `match`, pays more than 5x.

`whichever` puts `#[inline]` on every generated method. The generated-code
table in `RESULTS.md` shows its functions identical to the hand-written
ones, so the numbers above are its numbers.

Fat LTO closes most of that gap for the other crates, and it is a fair
thing to turn on in a release build anyway.

## When an enum pays

- **The set of types is closed**, and known where the enum is defined: a
  parser's node kinds, an interpreter's values, the shapes a renderer
  draws, the backends a service can be configured with, the messages of a
  protocol.
- **A hot loop visits many such values, and their methods are small.** The
  smaller the method and the fewer the variants, the larger the gain, up
  to an order of magnitude.
- **The values live in a collection.** Inline storage saves an allocation
  per element on the way in. Once the collection outgrows the cache, it
  also saves a trip to memory per element on every pass.
- **The trait needs what a trait object cannot offer:** `-> Self`, a
  `&Self` argument (`fn congruent(&self, other: &Self)`), associated types
  or consts, or generic methods. Or the holder needs `Clone`, `PartialEq`,
  `Hash` or serialisation. An enum has all of these, and `whichever`
  forwards them. When `self` and a `Self` argument hold different
  variants, you declare a [mismatch](glossary.md#mismatch) value: what to
  return then.
- **The code has to run without an allocator**, or with every value on the
  stack, as on `no_std` targets.
- **You want exhaustiveness:** a new kind of value should break every place
  that forgot to handle it.

## When it does not

- **The set is open.** Types from other crates, plugins, or anything a
  user of your library can add cannot be listed in an enum. A trait object
  is the right tool, or see the escape hatch below.
- **The variants differ a lot in size.** An enum is as large as its
  largest variant. One 200-byte variant next to 16-byte ones makes every
  element 200 bytes, and a `Vec` of them mostly padding. That costs more
  in memory traffic than the dispatch saves. Box the large variants
  (below).
- **The method bodies are large and the calls infrequent.** Then the
  dispatch costs the same either way, inlining gains nothing, and the
  trait object's flexibility is free.
- **A type is known at the call site after all.** A generic is cheaper
  than either. The enum's `From` impls make it easy to stay generic until
  the point where a value really has to be one of several.
- **Code size matters and the trait is wide.** Each method of the enum is
  a `match` over every variant, and with inlinable bodies each arm is a
  copy of the body. A vtable is one pointer per method.
- **You expect the 10x.** It is there for tiny methods over mixed
  collections of a few variants, and nowhere else. With more variants, or
  methods that cannot inline, the dispatch costs about the same either
  way, and memory decides more than dispatch.

## Patterns

**An escape hatch.** Give a closed set one extra variant for everything
else. The common kinds stay inline and fast, and a type nobody listed can
still get in. `whichever` forwards to a `Box<dyn Trait>` payload through
`#[dispatch(deref)]`, as long as the trait is dyn compatible:

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

A `deref` variant cannot take part in a method that returns `Self`,
because there is no way to put the result back behind the pointer.
Everything else works as for a plain payload.

**Grouping before a hot loop.** Sort or bucket a collection by variant, and
a random layout becomes a uniform one. That is worth more than any choice
of dispatch, when the loop runs often enough to pay back the sorting.

**Generic inside, enum outside.** Keep the inner loop generic over
`T: Trait`, and let the enum choose which instantiation to call once per
batch rather than once per element. The dispatch then costs nothing that
matters, and each instantiation runs at the floor.

## Getting the full benefit

- Hold the enum in a `Vec<Enum>`, not a `Vec<Box<Enum>>`. The point is the
  inline payload.
- Payload methods that should inline across a crate boundary need
  `#[inline]` too, or LTO. Rustc inlines a small, call-free function from
  another crate on its own, but not one that calls anything.
- Keep the variants close in size, and box the ones that are not.
- Group collections by variant where the program allows it.
- Measure on the machine that matters. Run the suite with
  `cargo bench -p whichever-benchmarks`, and write its tables with
  `benchmarks/scripts/report.py`. Adding a subject that mirrors your own
  trait takes a few lines; see the
  [benchmark README](../benchmarks/README.md).

## Why `whichever` and not a hand-written impl

The generated impl is the hand-written one, instruction for instruction.
`benchmarks/scripts/codegen.py` checks that for every subject.

What the attribute adds:

- The impl stays correct when the trait grows.
- It forwards the items people get wrong or leave out by hand: provided
  (default) methods, `Self` arguments, and associated types whose concrete
  types differ between variants.
- It works with a trait from another crate.

The other crates that do this are compared in
[the comparison](comparison.md).
