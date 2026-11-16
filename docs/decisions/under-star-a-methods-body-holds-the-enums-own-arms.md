# Under `*`, a method's body holds the enum's own arms

Status: accepted, 2026-11-15. Serves rule 1 of the [vision](../vision.md),
what the output is, and rule 3, what earns a shape. It answers the
reopening condition of [A partial block's method names its
variants](a-partial-blocks-method-names-its-variants.md), a body per
variant rather than one body for the rest, whose decision stands beside
this one; [A variant without a payload is left to the
fallback](a-variant-without-a-payload-is-left-to-the-fallback.md) stands
whole. Reopen if a surveyed project turns on a one-line body under `*`, or
has a variant that answers every method alike often enough that repeating
its arm is the cost.

## Context

A hand's forwarding `match` over an enum with exceptions has two kinds of
arm and no wildcard: the arms that forward, and the arms the enum answers
itself. A method that names its variants writes the first kind from its
list and makes its body `_ => body`, and both costs came from that
wildcard. It is on the side nothing checks: a variant added to the enum
and left off the list takes the body at run time, where the hand's `match`
is E0004. And it binds nothing: an exception whose arm reads its fields
opened with `let Self::Unknown(kind, data) = self else { unreachable!()
};`, which no hand writes.

Seven enums in five surveyed projects write this `match` by hand: mp4-atom
0.17.1's `Codec` (34 of 35 variants forwarded, `Unknown(FourCC, Vec<u8>)`
answering `Err`), `Any` (151 of 152) and `DrefEntry` (2 of 3), whose
`Unknown(kind, data)` arm encodes both fields; rsipstack 0.8.5's `Header`,
whose `Other(String, String)` answers from a field; hickory-proto 0.26.3's
`RData`, with three arms of its own in `BinEncodable::emit` and four in
`Display`; PathMap's `TaggedNodeRef`, thirty inherent methods beside a
unit `EmptyNode`; and Apache Iggy's `BenchmarkKindCommand`, ten methods
beside a unit `Examples`.

## Decision

Under `*`, a method of a partial impl block or of an inherent block may
have a body, and the body is one `match self` holding the arms the enum
answers itself. The attribute writes a forwarding arm for every variant
those arms do not name. The generated `match` is the forwarding arms, in
the enum's order, then the body's arms as written, with no wildcard: what
is written is the hand's `match` without the arms that only forward, and
what is generated is that `match` in full.

`*` keeps its meaning, every variant. It leaves no variant to a body for
the rest, which is why a body under it was reported; the only variants a
body can answer for there are the ones it names. The side left open to a
new variant is the side the compiler checks: a variant added to the enum
is forwarded, a payload without the impl is rustc's E0277, and a variant
with neither a payload nor an arm is an error on that variant, as before,
whose sentence now names the arm as the way around where the method can
take one. No build changes meaning: `*` with a body was an error, and a
body under a named list is still the arm for every other variant.

An arm takes the variants its pattern names by path at the top level,
`Self::Unknown(..)`, through a guard, an or-pattern, a reference,
parentheses or a binding `name @ ..`; a lone identifier is the variant of
that name. A variant an arm names is not forwarded, with a guard or
without. An arm that names none, `_` or a binding, is one sentence, since
it would leave nothing to forward; the form that has a wildcard is the
named list with a body. A body under `*` that is anything but one `match
self` is one sentence. The arms are the method's own: written as they
stand, in the declared return type, with no `with` and no conversion
around them, binding their fields as a hand's arm does under the method's
receiver, and where the arms are awaited in the future, so are they. An
arm under `#[cfg]` answers where the `cfg` holds and its variant is
forwarded where it does not, as a method written under `#[cfg]` is. A
trait method with a `Self` argument, which is matched beside `self`, and a
method over an enum whose every variant is under `#[cfg]`, which is
matched by place, take no arms of their own: each is one sentence, the
named list with a body writes both, and that list is still the way
around their variants without a payload.

## Evidence

- The seven enums above; none of the five projects is waiting to adopt.
- mp4-atom, probed on 0.7.3: a codec added to the enum and left off the
  34-name list compiles and returns `Err(UnknownCodec)` at run time, where
  the hand's `match` without the arm is E0004.
- hickory-proto, read at `v0.26.3`: `Update0(RecordType)` and `OPT(OPT)`
  each hold one payload that has the impl, and `RData` answers for both
  itself, so "holds a payload" and "is forwarded to" are different facts
  about a variant.
- `tests/expand/own_arms_under_star.rs`; `tests/behaviour/`,
  `own_arms_under_star`; `tests/ui/inherent_all_with_body.rs`,
  `own_arm_naming_no_variant.rs`,
  `own_arms_leaving_a_variant_without_a_payload.rs`,
  `own_arms_beside_a_self_argument.rs` and
  `own_arms_over_variants_all_under_cfg.rs`.

## Not chosen

- `*` with a body for the variants that hold no payload. It gives `*` a
  rest, chosen by how many fields a variant holds; a new variant without a
  payload takes the body silently; and it cannot make an exception of a
  variant that holds a payload, so hickory's `RData` would compile and
  answer differently from the hand. It writes three of the seven enums in
  one line each, where the arms take three. It can still be added: a body
  under `*` that is not a `match self` stays an error. The other order
  would not have been safe, since an arm naming a payload variant would
  have been dead code first and the answer afterwards.
- `* except Unknown` with one body. It keeps the check, but the body is
  handed no fields and is one answer for every exception, so it writes
  the same three; beside the arms it would be a second way to name an
  exception.
- Arms under a named list: no surveyed project writes it, and a body
  there has a meaning already.
- The exception as a payload, `Unknown(UnknownCodec)` over a struct with
  an impl of its own. It works with the plain attribute and changes a
  public variant and its serde shape, so it is the adopter's choice.
- One answer written on the variant for every method, which Iggy's
  `Examples` would want: with the arms it is repeated in each method.
