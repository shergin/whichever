# A place is handed over as the return type asks

Status: accepted, 2026-11-13. Serves rule 1 of the [vision](../vision.md),
the output is the hand's. Completes [An arm reaches its target through a
route](an-arm-reaches-its-target-through-a-route.md), which introduced
`field` and `value`. Reopen if a surveyed project hands a lift other than
`Some` or `Ok` a pattern-bound target by value or a routed place by
reference, or returns a reference through a type alias from such an arm.

## Context

`field = name` and `value` on an inherent method make the arm a place, not
a call. A place reached through `via` or `field` was borrowed as the return
type asks; a target the pattern binds, a plain payload, the marked field of
a variant with several or a dispatch struct's field, was handed over as the
binding, a reference under `&self`. So `#[dispatch(*, value)]` on `fn
handle(&self) -> Handle` did not compile on any of them, rustc's E0308,
expected `Handle`, found `&Handle`, where a hand writes `*inner`. acadrust
0.6.8's `ObjectType::intrinsic_handle` reads a `handle` over 35 tuple
variants and a struct variant `Unknown { handle, .. }`.

## Decision

One reading for every place. What takes the place is read off the
signature as written: the return type, or under `with = Some` or `with =
Ok` its first type argument. A reference borrows the place, `&inner.name`
through `via` or `field`, and `inner` for a target the pattern binds,
which under `&self` is already the reference. Any other named type takes
it by value, `inner.name` through `via` or `field`, and `*inner` for a
bound target under `&self`, `&mut self` or a pinned receiver. Where the
signature cannot say, a lift other than `Some` or `Ok`, either of the two
over a return type with no first type argument, a raw pointer return or
an `impl Trait` return on a dispatch struct, the arm is the target as it
stands: the binding for a bound target, the place by value for a routed
one. `self` by value hands the value itself; an `async` method awaits the
binding. Parentheses and a `macro_rules!` fragment's group around the
return type are looked through.

## Evidence

- A probe of 408 one-method cells, each with a hand-written twin: eight
  kinds of target, five receivers, sixteen return forms, run on 2026-11-13
  against 0.7.3. Every target the pattern binds failed in the same ten
  cells, where the hand writes `*inner`; trait dispatch had no mismatch.
  After the change the 35 cells whose return type says "value" compile,
  seven for each of the five bound kinds, and no cell that compiled
  stopped compiling. The cells still failing hand a lift other than `Some`
  or `Ok` a target whose parameter takes the other form, `by_val(inner)`
  under `&self`, which the signature cannot tell.
- `tests/behaviour/routes.rs`, `tests/behaviour/inherent_block_on_a_struct.rs`
  and `tests/behaviour/enums_with_every_variant_under_cfg.rs`; the goldens
  `tests/expand/marked_field_by_value.rs` and
  `tests/expand/payload_and_field_by_value.rs`.
- The planning note `notes/archive/bugs/marked-field-value-not-borrowed.md`.

## Not chosen

- Reading a bound target by value wherever no borrow is read, lifts
  included: `by_ref(inner)` on a payload or a marked field compiles and is
  the hand's arm, and would stop compiling so that `by_val(*inner)` could,
  with no surveyed case for either.
- Reading the lift's parameter: nothing is kept between expansions, and a
  function's signature is not in the tokens the attribute sees.
- Keeping the binding as it is for a payload, as 0.7.0 to 0.7.3 did:
  `value` then follows the receiver where `field` follows the return type,
  and a by-value return has no spelling.
- Resolving type aliases: the attribute sees tokens, so a return type that
  hides a reference behind an alias reads as a value, as it does for
  `field` and routes; the way around is to spell the reference.
