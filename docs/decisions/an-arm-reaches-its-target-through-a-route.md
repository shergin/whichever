# An arm reaches its target through a route

Status: accepted, 2026-11-05. Serves rules 1 and 3 of the
[vision](../vision.md), the output is the hand's and what earns a shape.
Supersedes the open directions B and C of [A variant without a payload is
left to the fallback](a-variant-without-a-payload-is-left-to-the-fallback.md),
whose decision stands. Reopen if a surveyed project needs a route with an
argument, a fallible step, or a method route under `&mut self`, each of
which is a limit today.

## Context

A plain variant's arm calls the trait on the payload; `deref` called it on
what the payload dereferences to. The surveys asked, from five directions,
for the same thing with a different last step: a field of the payload
(celestia, proof-of-sql's 114 arms, atuin, sqruff, mrml, quiche, rutui,
mlt-core's 62), what a method returns (memcache, rutui, otter, forest),
the payload itself handed to `Some` as an `Option<&dyn Trait>` (regexml,
screeps' 147 arms, pabal, clarabel), and a field of a variant with several
(atuin, sourmash). Each was "an expression applied to the binding before
the call", and one of them already existed under the name `deref`.

## Decision

One concept, the route: `*`, `.field`, `.0` and `.method()` in any
sequence, applied to the bound payload. It is written where a hand writes
it: on a variant or a field for every trait; as the `#[dispatch]` marker
on one field of a variant with several, which is the marker the struct
form already used; on the trait keyed by variant when the route differs
per trait; on an inherent method for that method alone. Where the route
names the target's type the call stays `<Payload as Trait>::method`; where
it cannot, a field or a method, the call is `<_ as Trait>::method(..)`,
qualified by the trait alone, which still keeps an inherent method of the
same name from taking over. Inherent dispatch gains `field = name` and
`value`, the two arms that call nothing.

Rules that keep rule 1: a route is a place expression or a method call
without arguments, so an arm stays one line; a fallible step is a limit; a
method route serves `&self` and `self`, since nothing borrows mutably
through a call; `-> Self` through a route is a limit, as it was for
`deref`; a variant with other fields gets no conversion; a target without
a nameable type is left out of the agreement checks.

## Evidence

- The planning note `notes/gaps/projection.md`, with the cases from five
  pages of surveys, and the review of 2026-11-05, section 3.6.
- spire_enum 1.2.0 settled the variant-side spelling as a marker on the
  field; ambassador's issue #12 (2019) asked for the per-trait form.
- A probe: `<_ as Trait>::method(&inner.field)` compiles, infers the type
  from the receiver, and does not pick an inherent method of the name.
- `tests/expand/routes.rs`; `tests/behaviour.rs`, `routes`;
  `tests/ui/route_method_on_mut_self.rs`, `two_marked_fields.rs`,
  `route_through_unknown_variant.rs`, `self_return_through_a_route.rs`.

## Not chosen

- A route keyed by trait on the variant: a trait's story in one place
  reads better, and the marker covers the common case without a key.
- Method-call syntax for every routed arm: it would let an inherent
  method of the trait method's name take over, which the fully qualified
  call exists to prevent.
- An arm template, the whole arm as the user's expression: a second place
  to write a body; confluent-kafka's 324 arms stay on the watch list.
