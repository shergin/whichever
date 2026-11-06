# Vision

Whichever implements a trait for an enum by forwarding to whichever variant
it holds. One attribute on the trait, one on the enum, and the enum has the
impl a careful hand would write: one `match` per method, a fully qualified
call per arm, `#[inline]`, and nothing else. The trait and the enum may live
in different crates, in any order, because nothing is kept between the two
expansions.

The goal is not the longest list of supported shapes. It is the forwarding
impl a hand would write, for every shape a real trait has, with nothing
added. Better a few strict rules than lots of features: that is how the
generated code stays the hand's and a limit stays one sentence.

That means three commitments:

- **The output is the hand's.** What the attribute writes is what you would
  have written, instruction for instruction, and it is checked against the
  hand-written impl in the expansion goldens, in the assembly, and in every
  benchmark table.
- **The trait and the enum may live anywhere.** The trait's tokens travel in
  a macro the trait exports. No cache, no registry, no state in the
  proc-macro server, so no order of compilation matters and no crate
  boundary is special.
- **Every limit is one sentence on the item.** A shape the attribute does
  not support is reported on the trait item or variant that has it, in one
  sentence, with the way around it where one exists, and the enum is still
  emitted so that error stands alone.

A dispatch macro usually grows by accretion: a flag per complaint, a cache
to make the convenient case work, a wrapper type to make a hard shape fit.
Whichever does not. The trait is read once, where it is declared, and the
rest follows from that:

- `#[dispatchable]` leaves the trait unchanged and emits its carrier macro,
  holding the trait's tokens, re-exported under the trait's own name.
- `#[dispatch(Trait)]` on the enum invokes that macro, which hands the trait
  and the enum to the implementation macro.
- The implementation macro writes the forwarding impl, the conversions and
  the agreement checks, and nothing it knows outlives the expansion.
- The same attribute on a struct forwards to one field; on an inherent
  block it forwards methods to named variants around a fallback body; on a
  trait impl block it forwards around the items written there.
- A restatement declares another crate's trait once, and `whichever::foreign`
  ships the restatements an enum most often needs.

Adoption may follow. It is never chased: no option per complaint, no parity
race with the other dispatch crates, no shape "for completeness".

## Who it is for

Crates that hold one of several types and pay for it by hand today: a
parser's node kinds, an interpreter's values, the backends a service can be
configured with, the messages of a protocol. The forwarding impl is the
price of the enum, paid again for every method and every variant; this
crate pays it.

Users of `enum_dispatch` second. The two attributes feel the same, the
traits it cannot handle work here, and nothing is carried between
expansions, which is where its failures came from. [When enum dispatch
pays](when-to-use-enum-dispatch.md) says when an enum is the right tool at
all; this crate takes no position on that beyond the measurements.

## The rules

Five rules, one axis each: what the output is, what the macro keeps, what
enters, how a limit reads, and what is true. Each names what holds it
today, because a rule without a mechanism that fails when it rots is a
wish.

1. **The output is the hand's.** One `match` per method, a fully qualified
   call per arm, `#[inline]`, and nothing a careful hand would not write: no
   trait object, no closure, no helper trait, no state. Where a hand would
   convert, `Into` for an overridden associated type, a rewrap for a `Self`
   return, so does the attribute, and nowhere else. A shape whose only
   spelling hides a cost, an allocation, a refcount, a conversion the
   signature does not name, is a limit, not a clever expansion.
   *Held by* the expansion goldens in `tests/expand`, the assembly
   comparison that `benchmarks/scripts/codegen.py` runs in CI against the
   hand-written impl, and the `hand` row of every benchmark table.
2. **Nothing is kept between expansions.** The trait's tokens travel in a
   macro the trait exports; the enum's expansion reads them and remembers
   nothing. A build is a function of the source alone, so the trait and the
   enum live in any crates, in any order, and under rust-analyzer.
   *Held by* the `smoke/` crates, which declare a trait in one crate and
   dispatch it from another, and by the carrier macro's name, a hash of the
   trait's tokens and their position, so that nothing has to be remembered
   to tell two traits apart.
3. **A shape earns support when real code has it.** A receiver, a return
   shape or an option enters when a surveyed project writes the forwarding
   by hand and no composition of what exists (a partial impl block, a
   restatement, `with =`, `call =`) expresses it. A shape that fails the
   test is a limit with a sentence, or an example of the composition; a
   shape refused for now sits on a watch list with the trigger that would
   promote it.
   *Held by* the gap notes in the planning notes, each naming its motivating
   project at a version and its acceptance test, and by the survey log.
4. **A limit is one sentence on the item.** Every unsupported shape is
   reported on the offending trait item or variant, in one sentence that
   says what is wrong and, where one exists, the way around it. The enum is
   still emitted, so that one error stands alone.
   *Held by* `tests/ui`, one compile-fail case per limit, with the wording
   pinned to stable's, and by a unit test of the macro crate that reads
   every sentence its sources write and fails when one has no snapshot.
5. **Claims are measured against the hand.** Every number in these docs
   comes from the benchmark suite, with machine, toolchain and date, called
   from a crate other than the enum's, without link-time optimisation. The
   reference is the hand-written `match`, never a competitor. Benchmarks are
   runtime only.
   *Held by* `benchmarks/RESULTS.md`, which `benchmarks/scripts/report.py`
   writes with its date, machine and toolchain, by the `agree!` tests
   that check every subject against the hand before anything is timed, and
   by `benchmarks/scripts/check_numbers.py`, which CI runs so that every
   number the README and the guide quote is one that file holds.

## Decisions

Choices made among real alternatives live in [decisions/](decisions/), each
with its evidence and the condition that would reopen it. Decisions may be
superseded. The rules may not.

## What it will not be

- Not a runtime. Nothing generated holds a trait object, a registry or a
  vtable. An enum that wants an open set holds a `Box<dyn Trait>` in a
  `deref` variant, which is the hand's answer too.
- Not a visitor or a type-level selector. A `match` that calls a method is
  what is written; mapping each variant to its payload's type as a generic
  argument is a different generator.
- Not a way to read another crate's enum. A restatement names its variants
  once, as it names a trait's signatures, and the impl is written for the
  real path, as a hand writes `impl LocalTrait for other::Either<L, R>`.
- No state between expansions, however convenient a case it would make
  work.
- No code a careful hand would not write: no allocation, no refcount, no
  conversion the signature does not ask for, hidden inside a generated arm.

## The name

Whichever variant it holds.
