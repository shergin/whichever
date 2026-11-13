# An enum that may be empty is matched by place

Status: accepted, 2026-11-12. Serves rule 1 of the [vision](../vision.md),
the output is the hand's: over an enum that may be empty a hand writes
`match *self` with `ref` bindings, and over every other enum `match self`.
Reopen if stable Rust accepts a `match` with no arms on a reference to an
uninhabited type; if a surveyed project has an enum that is empty in some
build although not every variant is under a plain `#[cfg]` (a variant
under `#[cfg_attr(.., cfg(..))]` is read as always present); or if the two
spellings turn out to confuse readers of expansions.

## Context

martin-core 0.17.0's `BackendSource` puts every variant under
`#[cfg(feature = ..)]`, so a build with its `_tiles` feature alone compiles
none of them, and its hand-written `dispatch_backend!` writes `match *self
{ .. }`, which compiles with no arm left. The generated `match self { .. }`
did not: a reference always counts as inhabited, so a `match` with no arms
on `&AnyShape` or `&mut AnyShape` is rustc's E0004, `type &AnyShape is
non-empty`, 43 errors in the survey's probe. The by-value `match self {}`
on the empty enum was always fine. Three directions: match by place for
every enum; match by place only for an enum that may be empty; keep
`match self` and add an arm for the empty build.

## Decision

The second. Where every variant of the enum is under `#[cfg]`, and the
receiver is a reference, `&self`, `&mut self`, `self: Pin<&Self>` or
`self: Pin<&mut Self>`, the `match` looks at the enum itself and the arms
borrow the payload: `match *self { AnyShape::Circle(ref __inner) => .. }`,
`ref mut` for an exclusive receiver, `*Pin::get_mut(self)` for a pinned
one. Such an enum is the only one that can be empty, and the macro sees
that from the variants' attributes without evaluating any `cfg`. Every
other enum keeps the expansion a hand writes today, `match self` with
default binding modes, and so do by-value and `Box<Self>` receivers and a
method with a `Self` argument, whose tuple scrutinee has the fallback arm
in every build. Trait dispatch, partial blocks and inherent blocks all go
through `MethodPlan::over`.

## Evidence

- The planning note `notes/bugs/all-variants-compiled-out-gives-e0004.md`,
  with the probe and martin-core's `dispatch_backend!`.
- `tests/expand/every_variant_under_cfg.rs`: the shared, exclusive,
  by-value and pinned receivers over an enum whose every variant is under
  `#[cfg]`.
- `tests/behaviour/enums_with_every_variant_under_cfg.rs`: enums with every
  variant off, which only have to compile, and with every variant on, which
  run.
- No other golden or snapshot changed, so the README's examples and the
  benchmarks' hand-written reference stand.

## Not chosen

- `match *self` with `ref` bindings for every enum: one spelling in the
  engine, but every expansion would move from the idiom a hand writes now
  to the one a hand wrote before default binding modes (Rust 1.26), for the
  sake of a rare shape.
- `match self` with an arm for the empty build,
  `#[cfg(not(any(..)))] __empty => match *__empty {}`: no pattern changes,
  but it is an arm no hand writes, and it repeats every variant's predicate
  in a second place.
