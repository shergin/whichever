# Variants that reach one type are not told apart

Status: accepted, 2026-11-17. Completes
[`impl Trait` returns: an enum forwards futures, a struct forwards them all](impl-trait-returns-are-forwarded-through-a-struct.md),
whose reopening condition spenso met and which stands. Serves rules 3 and 4
of the [vision](../vision.md), what earns a shape and a limit is one
sentence on the item. Reopen when a second surveyed project's variants
reach one type, or when a route can name the type it reaches, which would
let the attribute see that the targets agree.

## Context

Where every variant reaches a target of one type, a hand's `match` returns
a return-position `impl Trait`, since every arm is that type's, and
forwards a `Self` argument across variants. spenso 0.6.0's `DataTensor`
holds a dense and a sparse tensor, each with a `structure` of one type, and
forwards `TensorStructure` to it: four of the ten methods return `impl
Iterator`.

The attribute refused the `impl Trait` "because every variant returns a
different opaque type", which is not true of this enum, and asked for a
`mismatch` where the trait's own default would have compared the two
values.

## Decision

The limits stay, and the sentences say what holds.

- An enum does not dispatch a return-position `impl Trait`: the arms of a
  `match` return one type, and the attribute cannot tell that the
  variants' opaque types are one. A field's or a method's type cannot be
  read off the payload, so variants routed to one type look like any
  others. The sentence names the method written by hand as the way where
  they do agree.
- A method with a `Self` argument still asks for the result of two
  different variants. Where the trait gives the method a body, the
  sentence names the third answer, the trait's default, which calls the
  forwarded methods on both values and so looks at them, where a
  `mismatch` expression cannot.

For spenso that leaves four methods written in the partial block, through
the accessor the enum already has.

## Evidence

- Issue #32, with the three forms a hand writes and what compiles today.
- `tests/ui/rpitit.rs`, `rpitit_nested.rs`, `impl_into_future.rs` and
  `self_argument_of_a_provided_method_without_mismatch.rs` pin the
  sentences; `tests/behaviour/`, `variants_reaching_one_type`, runs the
  partial block that writes the `impl Trait` methods and leaves the `Self`
  argument to the default.

## Not chosen

- An option saying that the variants reach one type: a claim the
  attribute cannot check, for four methods of one project.
- Letting rustc judge every `impl Trait` on an enum: variants of
  different types, the common case, would lose their sentence for "match
  arms have incompatible types" inside the expansion.
- Comparing the payload types as written and forwarding where they are one:
  no surveyed project holds one type in two variants, and spenso's
  payloads differ; only what they are routed to agrees.
