# The macro crate parses with `syn` 3

Status: accepted, 2026-11-12. Serves rule 1 of the [vision](../vision.md),
the output is the hand's, by leaving it alone: the parser changes and no
expansion does. Reopen if a later `syn` major reaches the common proc-macro
trees the way 3 has; if a surveyed project that weighs the dependency
builds `syn` 2 only and would gain `syn` 3 from `whichever` alone; or if a
surveyed project declines `whichever` over `syn` itself in a build that has
none, which is the case for a parser of our own.

## Context

The macro crate's dependency tree is fixed: `proc-macro2`, `quote`, `syn`,
and `proc-macro-crate` behind a feature. Its `syn` was 2. `syn` 3.0.0
shipped on 2026-07-18, and projects surveyed on 2026-11-12 count a second
`syn` in their build as a cost of taking a proc macro, before any shape is
discussed. Three directions: stay on `syn` 2, move to `syn` 3, or build
against either behind a feature.

## Decision

`whichever-macros` depends on `syn` 3, with the features it had: `full`,
`visit` and `visit-mut`. Nothing else in the tree changes, and the minimum
supported Rust version stays 1.88; `syn` 3 asks for 1.71. No expansion and
no sentence changes.

The port answered fifteen compile errors in five modules, each one a change
of `syn`'s tree and none a change of ours:

- A receiver's shorthand is `ReceiverKind` now, where `syn` 2 made up a
  type for `&self`; `classify_receiver` reads `self`, `&self` and
  `&mut self` by their spelling and a receiver with a type by that type.
- `TraitBound` carries `maybe` in place of `modifier`, `Signature` carries
  a three-way `Safety` in place of `unsafety`, and a parameter's default is
  one `Option` of a tuple.
- An impl's `!` moved from the trait's tuple into `ImplModifiers`; the
  negative-impl sentence reads it there and points at the same token.
- Every `Type` variant holds attributes. The recognisers of `Self`,
  `Self::Name` and a wrapper's last segment ignore them, and a projection
  rewritten through a trait parameter keeps them.

The rest compiled as it was: every `parse_quote!` and `Spanned` site, the
three `Visit` and two `VisitMut` implementations the engine has,
`Speculative`, and `ImplItem::Verbatim` for a method without a body.

## Evidence

- The survey of 2026-11-12: uutils/coreutils at `d8966c1` pins
  `syn = "3.0"`, builds `syn` 3 only for `uucore` and every standalone
  utility, and its `deny.toml` denies duplicate versions with `syn` 2 on
  the skip list. purecrypto builds `syn` 3 only; toasty and lofty have
  macro crates of their own on `syn` 3.
- rustzx #28 is reported to have dropped `enum_dispatch` for being on
  `syn` 2 while its new dependencies use `syn` 3; fjall-rs/lsm-tree#323
  wants to drop it as the only reason the crate builds `syn` at all.
- On crates.io on 2026-11-12, `serde_derive` 1.0.229, `thiserror-impl`
  2.0.21, `tokio-macros` 2.7.2, `async-trait` 0.1.92, `clap_derive` 4.6.7,
  `darling_core` 0.24.1, `futures-macro` 0.3.34 and `bytemuck_derive`
  1.12.1 depend on `syn` 3. `zerocopy-derive` 0.8.63, `strum_macros`
  0.28.0, `derive_more-impl` 2.1.1, `pin-project-internal` 1.1.13,
  `tracing-attributes` 0.1.31, and `enum_dispatch`, `ambassador` and
  `delegate` are on `syn` 2. A tree with a current `serde_derive` or
  `thiserror` in it builds `syn` 3 already.
- All 32 expansion goldens under `tests/expand/` and all 120 `.stderr`
  snapshots under `tests/ui/` are unchanged, compared by their tools and
  not regenerated.
- `tests/behaviour/lifetimes_and_generic_enums.rs` runs the one path the
  port moved that no golden held: a trait's lifetime on `&'a self` and
  `&'a mut self`, replaced by the enum's.
- `cargo tree -p whichever -e normal,build`, with the default features and
  without them, lists `syn` 3.0.10 and no `syn` 2.

## Not chosen

- Staying on `syn` 2: no work, and one crate fewer for a tree that is
  still wholly on `syn` 2. That tree is the shrinking one, and the cost
  lands on the projects that moved, coreutils among them, whose
  `deny.toml` counts it.
- Either `syn` behind a feature: Cargo features are additive, so a graph
  that turns both on needs both to compile together; the changed sites
  become two code paths, each with its own goldens to hold equal, in a
  crate whose first rule is one output.
- A parser of our own, with no `syn` at all, which is what lsm-tree's issue
  asks of a dispatch crate: the macro reads whole traits, impl blocks with
  bodies and every type a signature can name, so the parser would be a
  large part of `syn` rewritten, for builds that have no `syn` today. No
  surveyed project has declined `whichever` for that reason yet.
