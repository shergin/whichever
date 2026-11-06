# A trait item's `cfg` is decided in the trait's crate

Status: accepted, 2026-11-05. Serves rule 2 of the [vision](../vision.md),
nothing between expansions, whose other half is that a build is a function
of each crate's own source and configuration. Reopen if rustc lets a
macro-expanded exported macro be named by an absolute path from its own
crate, which would make a gate per predicate the smaller mechanism.

## Context

A `#[cfg(feature = "crypto")]` on a trait method is true or false in the
trait's crate. The carrier carried the attribute to the enum's crate, where
the generated method was compiled in or out by that crate's features, which
has no such feature (`unexpected_cfgs`) or a different one, and the impl
then lacked a method the trait had (E0046), or had one it did not. fvm's
`CryptoOps::verify_signature` is the surveyed case; a two-crate probe
reproduced both errors. Three directions: carry the attribute, which is the
defect; a gate per predicate, two exported macros of one name under the
predicate and its negation, with the generated item written inside the
gate, which is ambassador's design; one carrier per combination of the
distinct predicates, each under its combination's `cfg`.

## Decision

The carrier is written once per combination. Each definition is under
`#[cfg(all(P, not(Q), ..))]` for its combination and holds the items that
exist under it, with their `cfg` attributes taken off; rustc compiles
exactly one definition of the name, and the enum's crate reads a trait
whose items are the ones that exist. Nothing in the protocol changes, and
a carrier read by an older `implement!` is right too, since the items it
sees are the ones there are. A trait with more than four distinct
predicates, sixteen carriers, is reported on the fifth, with the way
around it. A variant's `#[cfg]` stays on its arm, since the enum expands
in its own crate.

## Evidence

- The probe of 2026-11-05 in the review notes: the gate form compiles
  clean across two crates with the feature on and off, and the carried
  attribute gives `unexpected cfg condition value` and E0046; the
  combination form is the same decision made at the carrier instead of
  the item.
- The gate form was implemented first and failed on
  `macro_expanded_macro_exports_accessed_by_absolute_paths`: a gate is a
  macro-expanded `#[macro_export]` macro, and `$crate::gate!` is an
  absolute path to it, which rustc refuses whenever the trait and the enum
  share a crate. ambassador's gates are reached through a module named
  after its macro, which a carrier named after the trait cannot have.
- `tests/behaviour/cfg_on_trait_items.rs`; `smoke/traits`' `Gated`,
  dispatched from `smoke/enums` under `unexpected_cfgs = "deny"`;
  `tests/expand/cfg_on_trait_items.rs`; `tests/ui/too_many_cfg_predicates.rs`.

## Not chosen

- Carrying the attribute: the defect.
- A gate per predicate: linear in the predicates, but unreachable by path
  from the trait's own crate.
- Deciding at the enum which crate the trait is in, through
  `CARGO_CRATE_NAME`, and carrying the attribute when it is the same: two
  mechanisms for one thing, and a name two crates may share.
