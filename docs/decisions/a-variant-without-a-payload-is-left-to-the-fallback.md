# A variant without a payload is left to the fallback

Status: accepted, 2026-11-02 (0.4.0). Serves rule 3 of the
[vision](../vision.md), what earns a shape. Its reopening condition was
met on 2026-11-05, and the per-trait projection and the field forwarding
it watched shipped as routes, see [An arm reaches its target through a
route](an-arm-reaches-its-target-through-a-route.md); the decision below,
that a variant without a payload takes the fallback body, stands.

## Context

Trait dispatch requires every variant to be `Name(Payload)`. Real data
enums are not: proof-of-sql's `ColumnBounds` has a unit variant `NoOrder`
beside payload variants, and `Column` has multi-field variants whose trait
impls use one field. Three directions: accept any shape for the variants a
method does not forward to; a per-trait projection option; field forwarding
for inherent dispatch.

## Decision

The first. Inherent dispatch accepts a variant of any shape that a method
does not name: the variant takes the fallback body, and `*` over such a
variant is an error on that variant. Trait dispatch keeps the
`Name(Payload)` requirement, reported under the variant's own `#[cfg]`. No
opt-in: a bare `#[whichever::dispatch]` on a data enum with unit variants
now registers it for inherent dispatch.

## Evidence

- The planning note `notes/gaps/projection.md`, with the cases from five
  pages of surveys.
- `tests/ui/unit_variant.rs`, `tests/ui/struct_variant.rs` and
  `tests/ui/inherent_unknown_variant.rs` pin the sentences.

## Not chosen

- A per-trait projection on the dispatch attribute: covers proof-of-sql,
  celestia and memcache with one rule, but a `-> Self` through a projection
  cannot be rebuilt, and the syntax (keyed by variant on the trait, or by
  trait on the variant) is undecided. Watched.
- Field forwarding for inherent dispatch: sqruff's case needs a per-variant
  override as soon as a second project (twitch-irc) is read. Watched.
