# A mutable borrow takes a route of its own

Status: accepted, 2026-11-17. Completes
[An arm reaches its target through a route](an-arm-reaches-its-target-through-a-route.md),
whose reopening condition, a method route under `&mut self`, confluent-kafka
and kithara met, and supersedes one of its rules: "a method route serves
`&self` and `self`, since nothing borrows mutably through a call". Serves
rules 1 and 3 of the [vision](../vision.md), the output is the hand's and
what earns a shape. Reopen if a surveyed project forwards through a method
that returns its target by value and would be served by a route that
borrows the temporary, or needs a third route for `self` by value.

## Context

A route that ends in a method call yields a value, which the arm hands over
as it is: the attribute cannot see what the method returns. The record took
that to mean that a mutable borrow cannot go through a call at all, and
refused every `&mut self` method over a method route, naming a field route
as the way around. Both halves were wrong for a getter that lends mutably,
`fn data_mut(&mut self) -> &mut Data`: its result is the mutable borrow,
and the hand's arm, `inner.data_mut().to_send(version)`, compiles.

confluent-kafka 1.0.0 writes 324 such arms over two enums, and
`MetadataResponse::data_mut` drops a cached value before it lends, so a
route to the field would forward and silently skip that. kithara-play
forwards three traits through the getter pair of its field's type,
`source()` for `&self` methods and `source_mut()` for seven `&mut self`
ones.

## Decision

A method route lends as its method does, so one method serves one way of
borrowing, and a mutable borrow takes a route of its own.

- `via_mut = route` names it, beside `via`, on a variant or on a field, and
  `via_mut(name = route)` beside a trait, where `via(name = route)` goes.
  A method that borrows the target mutably, by `&mut self`, by `self:
  Pin<&mut Self>` or in a `&mut Self` argument, follows it; everything else
  follows `via`. Each `Self` argument takes the route of its own borrow, so
  `fn absorb(&mut self, other: &Self)` is
  `absorb(inner.source_mut(), other.source())`.
- A route written on a method's own mark is that method's alone, and is
  followed whatever the method borrows: `#[dispatch(*, via = .data_mut(),
  call = to_send)]` on a `&mut self` method is the hand's arm.
- Without either, a mutable borrow through a method route is still one
  sentence on the variant, which now names `via_mut`: the common case is a
  getter that lends shared, which rustc would report as a type mismatch on
  every method.
- A trait's `via` for a variant replaces both of the variant's own routes,
  so that a mutable borrow never follows the variant's route to another
  target; a trait's `via_mut` replaces the variant's `via_mut` alone.
- A place arm, `value`, hands a method's result over as it is, as a call
  does, for either borrow.

What a route's method returns stays rustc's to judge, and its judgement is
located on the method's name in the route.

## Evidence

- Issue #27, with the two shapes and the three ways around them compiled.
- `tests/behaviour/`, `mutable_routes`: a struct and an enum over a getter pair,
  each `Self` argument by its own route, a pinned receiver, and the place
  arms; `tests/expand/routes.rs`.
- `tests/ui/route_method_on_mut_self.rs` and
  `route_method_under_a_pinned_receiver_or_a_mut_argument.rs` pin the
  sentence; `route_method_result_does_not_fit_the_receiver.rs` pins where
  rustc's error lands.

## Not chosen

- Dropping the refusal and letting rustc judge every method route: a field
  that names the shared getter alone would be rustc's E0308 once per
  `&mut self` method, where one sentence names the option.
- A mark on the route's step saying that its method lends mutably: the
  shared and the mutable getter are two methods, so two routes are what
  there is to say.
- `via_mut` on a method's own mark: the mark is one method's, whose
  receiver is beside it.
- A route that borrows what its method returns by value, `&inner.parsed()`
  under `&self` (issue #29). spacepackets 0.19.0, the one project that
  writes it, calls methods of the struct itself, which no route starts at,
  so it would stay by hand. The arm is rustc's E0308, on the route.
