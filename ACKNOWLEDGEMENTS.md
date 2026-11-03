# Acknowledgements

One specific lesson per credit.

## The crates that came before

- [`enum_dispatch`](https://crates.io/crates/enum_dispatch) showed that a
  macro can write the forwarding impl, and gave the two-attribute shape
  that `whichever` keeps. Its process-global cache, and the three failures
  that follow from it, decided that the trait's tokens would travel in a
  macro instead.
- [`ambassador`](https://crates.io/crates/ambassador) restates a trait
  another crate owns, the precedent for `remote =`, and exports a macro per
  trait at the crate root, which showed the name collision that
  `whichever` hashes around.
- [`portrait`](https://crates.io/crates/portrait) fills an impl from a
  macro the trait emits, the same carrier idea applied to a different
  problem.
- [`delegate`](https://crates.io/crates/delegate) forwards exactly the
  methods you list, which is the measure of how much a user will write to
  keep an impl honest, and what `whichever` had to make unnecessary.
- [`enum_delegate`](https://crates.io/crates/enum_delegate) rewraps `-> Self`
  and unifies differing associated types in a wrapper enum; its random
  macro names and its omitted `#[inline]` are two things the benchmarks
  measured and this crate does differently.
- [`delegation`](https://crates.io/crates/delegation) forwards through a
  chain of `Either`s, which is where the benchmarks learned what walking a
  chain costs at 32 variants.
- [`serde`](https://crates.io/crates/serde)'s `#[serde(remote = "..")]` is
  the word and the shape for restating a type of another crate.

## The shape of these documents

The vision with a mechanism per rule, the decision records with a reopening
condition and the private planning notes follow the author's three earlier
projects, topos, malevich and baton, each of which refined the practice for
the next.
