# Compatibility

## What you need

- **`no_std`.** `whichever` is `#![no_std]`, and it has no dependencies
  beyond its own proc-macro crate.
- **Rust 1.88 or later.** That is the minimum supported Rust version.
- **`whichever` in both crates.** The crate that owns the trait and the
  crate that owns the enum both depend on `whichever`.

## Dependencies and the `renamed-dependency` feature

You can rename the `whichever` dependency in your `Cargo.toml`. That
works under the default feature `renamed-dependency`, which looks the
new name up through `proc-macro-crate`.

Without that feature, the macro crate's dependency tree is
`proc-macro2`, `quote` and `syn`, and nothing else. The `syn` is `syn` 3,
the one `serde_derive` and `thiserror` build. So a tree that has already
moved to it gains no second `syn`.

One case needs the feature off. A crate that `proc-macro-crate` itself
depends on, such as `toml_edit`, takes `whichever` with
`default-features = false`. With the feature on, that crate would depend
on itself through `proc-macro-crate`, and Cargo rejects the cycle.

## Versions across crates

Before 1.0, the trait's crate and the enum's crate use the same minor
version of `whichever`.

The reason is the [carrier macro](glossary.md#carrier-macro) (see the
next section). It is written by the `whichever` that the trait's crate
builds with, and read by whatever `whichever` the enum's crate builds
with. To Cargo, two minor versions of a 0.x crate are two different
crates, and one dependency graph may hold both. So:

- a newer `whichever` reads every older carrier macro;
- an older `whichever` reading a newer carrier macro gives a
  one-sentence error.

## What `#[dispatchable]` adds to your crate

`#[dispatchable]` leaves a carrier macro next to the trait. This is how
an enum reaches the trait's signatures, even from another crate.

Where it lives depends on the trait's visibility:

- **A `pub` trait** adds one `#[doc(hidden)]` exported macro to the
  crate root.
- **A trait of any other visibility** keeps its macro in its own module,
  and adds nothing to the crate root.

What it holds:

- the trait's signatures, and the attributes the generated items carry;
- and nothing else: no default body, no doc comment, no bound.

The fine print:

- **`#[cfg]` inside the trait.** A trait with items under `#[cfg]`
  defines the macro once per combination of its predicates, each under
  that combination's `cfg`. So exactly one definition of the name is
  compiled.
- **The name is deterministic.** The macro is named after the trait,
  with a hash of the trait's tokens and their positions. Nothing else
  goes into the name, so an expansion is the same on every build and on
  every machine.
- **`#[dispatchable(name = ..)]`** replaces the hash in the name. You
  need it when one `macro_rules!` emits the same trait twice.
