# Benchmarks

How fast is a trait method call on an enum that `whichever` implements? This
suite compares it with the `match` you would write by hand, with a
`Box<dyn Trait>`, and with the enums the other dispatch crates generate.

The answer for the machine the suite last ran on is in
[RESULTS.md](RESULTS.md). This page says what is measured and how, so you
can trust the numbers, reproduce them and extend them.

## What is compared

| Subject | What it is |
| --- | --- |
| `hand` | `impl Trait for Enum` written by hand: a `match`, one fully qualified call per arm, `#[inline]` on every method. The reference row; every ratio is against it. |
| `whichever` | `#[whichever::dispatchable]` on the trait, `#[whichever::dispatch(Trait)]` on the enum. |
| `enum_dispatch` 0.3 | `#[enum_dispatch(Enum, ..)]` on the trait, `#[enum_dispatch]` on the enum. |
| `ambassador` 0.5 | `#[delegatable_trait]` on the trait, `#[derive(Delegate)] #[delegate(Trait)]` on the enum. |
| `enum_delegate` 0.2 | `#[register]` on the trait, `#[implement(Trait)]` on the enum. |
| `delegation` 0.5 | `#[delegate]` on the trait, `#[delegate(derive(Trait))]` on the enum. |
| `delegate` 0.13 | `delegate! { to match self { .. } { fn ..; } }` inside a hand-written impl block, every signature listed again. |
| `box_dyn` | `Vec<Box<dyn Trait>>`: the dynamic dispatch an enum replaces. A `Vec<&dyn Trait>` loads the same fat pointer and vtable slot, so it is not a separate subject. |
| `monomorphic` | A `Vec` of one concrete payload type calling its method directly: no dispatch at all, the floor. |

A "subject" here is one row of this table: one way of making the call.

Every subject dispatches the same trait over the same 32 payload types,
defined once in [`subjects/`](subjects/src/lib.rs). Each payload holds two
`u64`s. So an enum is 24 bytes at every variant count, and a `Box<dyn>` is
a 16-byte fat pointer plus a 16-byte heap object. Each payload's method
body is a couple of integer operations, different for every payload, so no
two arms of a `match` compile to the same code.

The trait has four methods:

- a `&self` method returning a value,
- a `&mut self` method,
- a method with a default body, which half the payloads override,
- a `-> Self` method.

`ambassador` and `delegation` cannot forward `-> Self`, and a trait object
cannot return `Self`. So those subjects skip that one method.

## What is measured

Everything is one loop over a collection, timed by [criterion] with
`Throughput::Elements`, and reported as nanoseconds per element. The
variables are the collection, its layout (the order of the variants in it)
and the subject.

**`dispatch`** is the main matrix. It times one `&self` call per element
over 64 Ki elements (Ki is a binary thousand, 1024; Mi below is a binary
million). That is 1.5 MiB of enums, walked in order. The prefetcher (the
part of the CPU that loads memory ahead of a sequential walk) keeps memory
out of the measurement, so the `uniform` rows cost what they would inside
L1, the fastest cache. The matrix covers every subject, at 2, 4, 8, 16 and
32 variants, in two families and three layouts.

The two families:

- The `trivial` family marks the payload methods `#[inline]`. When the
  optimiser can see through a subject, it folds the method into the loop.
  This is where enum dispatch wins by a lot, and where `enum_dispatch`'s
  own benchmark gets its "up to 10x".
- The `opaque` family marks the payload methods `#[inline(never)]`. Every
  element then costs one real call, whatever the subject, and what is left
  is the cost of choosing which function to call. This is the fair
  comparison with a vtable (the table of function pointers behind a trait
  object), which can never inline.

The three layouts:

- `uniform` holds one variant in every element: the best case for any
  branch predictor (the part of the CPU that guesses which way a branch
  goes).
- `round_robin` cycles through the variants in order: a fixed period that
  a branch predictor may or may not learn.
- `shuffled` draws each element's variant at random. This is the mixed
  collection dynamic dispatch exists for. The branch is unpredictable, for
  a `match` and a vtable alike, because the collection is too long for the
  predictor to memorise.

**`methods`** takes the 8-variant `trivial` enums in the `shuffled` layout
and times each kind of method: `&self` returning a value, `&mut self`, the
default-body method and `-> Self`.

**`memory`** is where the enum's inline payload matters. It has two parts:

- `scattered` sums over collections of 1 Ki to 4 Mi elements that were
  shuffled after being built. So each `Box<dyn>` points at an unrelated
  heap address, as it would in a program that has been running for a
  while. An enum carries its payload with it and does not notice. Every
  element holds the same variant, so the branch is predicted for both and
  only memory is left to differ. With random variants, the predictor would
  learn the short collections and not the long ones, and that would read
  as a memory effect.
- `build_and_drop` builds a 1024-element collection, which for an enum is
  no allocation at all.

**Generated code.** `scripts/codegen.py` compiles
[`examples/codegen.rs`](examples/codegen.rs) to assembly. That file has one
exported function per subject, family, variant count and method, each
calling the subject's method once. The script then compares every function
with the hand-written one, after renaming labels and symbols. A subject
whose function is identical to `hand`'s generates the same machine code,
cross-crate inlining included.

The script also names the shape of each function:

- a compare chain (a sequence of compare-and-branch steps),
- a jump table (an array of code addresses indexed by the variant),
- an indirect call through a vtable,
- or a single out-of-line call, when the generated method was not inlined
  into the caller at all.

## Why it is built this way

**The loops are in a different crate from the enums.** `dispatch-subjects`
owns the traits, payloads and enums; `whichever-benchmarks` owns the loops.
That is how a dispatch enum is used: a library defines it, an application
calls it.

Without link-time optimisation (LTO), rustc inlines a function from another
crate only when it is generic, marked `#[inline]`, or small *and free of
calls*. A generated `match` always contains calls. So a subject that omits
`#[inline]` leaves every call out of line, and that shows in both the
numbers and the assembly. `enum_delegate`, `delegation` and
`static-dispatch` omit it. With fat LTO,
`cargo bench -p whichever-benchmarks --profile bench-lto` shows how much
of the difference it recovers.

**The collection is longer than the predictor's memory.** The loop replays
the same sequence of variants every iteration. A branch predictor with a
long history learns a random sequence of a thousand elements. Over 1024
elements, an M1 Pro ran the `shuffled` layout at 16 and 32 variants nearly
as fast as a predictable one, which is no measurement of random dispatch.
At 64 Ki elements, the cost of the random layout is within a few percent
of what it is over a million, while the collection still fits in L2.

**The only variable is the subject.** Each subject is one invocation of
the same `family!` macro with that subject's attributes. The trait, the
payload impls, the enums and the adapters the loops call are generated
from the same tokens for all of them. The adapters are `#[inline(always)]`,
so the loop sees exactly the subject's own method and whatever inlining
it allows.

**Nothing is folded away, nothing is over-protected.**

- Payload values come from a seeded generator, so no result is a
  compile-time constant.
- The slice is passed through `black_box` once per iteration. The results
  are reduced with a wrapping integer add, which is associative. So the
  compiler is free to inline, unroll and vectorise exactly as it would in a
  real loop.
- Applying `black_box` to every element, as `enum_dispatch`'s benchmark
  does, would forbid the optimisations enum dispatch exists to enable.
- Payload methods have distinct bodies. Arms that differ only by a
  constant would let the optimiser turn a whole `match` into one select
  chain, which no real trait allows.

**The dynamic baseline gets a fair run.** The boxes in the `dispatch`
matrix are allocated in element order, the friendliest heap layout a
`Box<dyn>` can have; the `memory` group is where that assumption is
dropped. The trait is dyn compatible (what Rust used to call object
safe), the vtable methods are the very same
functions the enums call, and nothing lets the compiler see a box's
concrete type.

**Two families, not one.** Any comparison of enum dispatch with a vtable is
dominated by one question: does the payload method get inlined? Measuring
both with inlining possible and with it ruled out separates "the compiler
could see through it" from "the dispatch itself is cheaper". The two
answers are different.

## Running

```bash
cargo bench -p whichever-benchmarks
```

This runs everything in about a quarter of an hour. `criterion` writes its
own HTML report to `target/criterion/report/index.html`. Useful
variations:

```bash
cargo bench -p whichever-benchmarks -- --quick                       # until significance, a few minutes
cargo bench -p whichever-benchmarks --bench dispatch -- 'dispatch/opaque/shuffled'   # one table
cargo bench -p whichever-benchmarks -- 'whichever|hand'              # two rows of every table
cargo bench -p whichever-benchmarks --profile bench-lto              # with fat LTO
cargo bench -p whichever-benchmarks -- --save-baseline before        # then --baseline before, after a change
```

Then run the two scripts, which need only Python 3:

```bash
python3 benchmarks/scripts/codegen.py     # compare generated code, write target/codegen/
python3 benchmarks/scripts/report.py      # write benchmarks/RESULTS.md from both
```

Or run all three in one go with
`benchmarks/scripts/run.sh [criterion args]`.

`cargo run -p whichever-benchmarks --release --example codegen` prints the
size of the types.

Run on a quiet, plugged-in machine. The report prints the widest 95%
confidence interval of each table, so a noisy run is visible.

`cargo test -p dispatch-subjects` checks, before anything is timed, that
every subject returns exactly what the hand-written enum returns, for every
method.

## Reading the results

Every number is a median over criterion's samples, in nanoseconds per
element; lower is better. The ratio to the `hand` row is in parentheses.
Rows that are statistically the same as `hand` differ by a few percent
either way. The confidence interval under each table is the yardstick.

What the tables show, and why:

- `hand`, `whichever`, `enum_dispatch`, `ambassador` and `delegate`
  generate the same `match` with `#[inline]`, and the generated-code table
  says they compile to the same instructions. Any difference between those
  rows is noise.
- Strategies without `#[inline]` cost one extra call per element across a
  crate boundary. That shows in the `opaque` family and is gone under LTO.
  `enum_delegate` gets inlined anyway while its method is small and
  call-free after the payload methods are inlined into it. That is why it
  keeps up in the `trivial` family until 32 variants. It falls behind on
  `scale`, where its `try_into().expect(..)` on the argument leaves a call
  in the body.
- `delegation` does not generate a `match`. It converts the enum to a
  nested `Either` chain. The generated-code table shows an out-of-line call
  in every cell, and the tables show what walking the chain costs as
  variants grow.
- Against `Box<dyn>`, the `trivial` family shows what inlining buys, the
  `opaque` family what the dispatch alone buys, and the layouts show where
  the branch predictor stops helping either of them.
- A jump table at 16 or 32 variants is an indirect branch too. Predicted,
  it costs a little more than the vtable call it replaces; unpredicted,
  both pay the same. Below 16 variants with out-of-line bodies, the `match`
  is a chain of compares, which a random variant can send the wrong way
  more than once per element. So there the vtable call is the faster.
- With up to 8 small arms, LLVM compiles the `match` into branch-free
  select code that evaluates every arm. So the `trivial` family is slower
  at 8 variants than at 16 or 32, where a jump table takes over. That is
  the compiler's choice for every `match`-based subject alike, and no
  choice a vtable can make.
- The `memory` tables are where the enum's inline payload shows. A
  scattered `Box<dyn>` collection pays for memory on every element once
  its heap objects no longer fit in the cache. Building one pays an
  allocation per element.
- `monomorphic` is the floor. The distance from it is the price of having
  a choice at all. Where it reads `<0.01`, the optimiser folded the whole
  loop into a constant.

These are numbers from one machine and one toolchain. The pattern stays
the same across machines; the ratios move.

## Adding a subject

1. Add the crate to [`subjects/Cargo.toml`](subjects/Cargo.toml).
2. Add a module to `subjects/src/` with two `family!` invocations,
   `trivial` and `opaque`. Give each the subject's attributes for the
   trait and for the enums, and a subject name for the reports; copy
   [`whichever.rs`](subjects/src/whichever.rs). Every trait and enum name
   has to be unique in the crate, because `enum_dispatch` keeps a
   process-wide cache keyed by bare identifier, and `ambassador` exports a
   macro named after the trait at the crate root. `resize: no` leaves out
   the `-> Self` method.
3. List the module in `subjects/src/lib.rs`, in the `agree!` tests there,
   in the `for_each_subject!` macros in [`src/lib.rs`](src/lib.rs), in
   `examples/codegen.rs` and in the `SUBJECTS` lists of the two scripts.

[criterion]: https://github.com/bheisler/criterion.rs
