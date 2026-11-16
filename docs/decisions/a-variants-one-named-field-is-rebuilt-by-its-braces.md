# A variant's one named field is rebuilt by its braces

Status: accepted, 2026-11-15, in the review of 2026-11-15, without a
surveyed project asking. Serves rule 1 of the [vision](../vision.md), the
output is the hand's, and follows [A named field is not rebuilt under a
wrapper](a-named-field-is-not-rebuilt-under-a-wrapper.md). The owner may
reverse it. Reopen if a surveyed project has a variant whose one field is
named and marked and needs a method returning `Self` refused there, or if
the rule for a struct with one named field changes.

## Context

A variant that holds its payload in a named field, `Named { #[dispatch]
inner: Square }`, forwards to that field. Until this review the attribute
read it as a field beside others, the shape of `Node { id: u32,
#[dispatch] data: Data }`, so a method returning `Self` was refused with
"cannot be built around the other fields of `Named`", fields the variant
does not have. A struct with one named field already had the other rule:
`-> Self` is rebuilt by its braces, and a `Self` under an `Option` or a
`Result`, which a path has to build, is a sentence naming the tuple form.
Two directions: keep the variant's limit with a sentence that is true, or
give the variant the struct's rule.

## Decision

The second. A variant whose one field is named and marked is rebuilt by
its braces for `-> Self`, `Named { inner: value }`, as a hand writes it
and as a struct with one named field is. Under `Option<Self>` or
`Result<Self, E>` it is a sentence on the payload with the tuple variant,
`Named(Square)`, as the way around, as on the struct, for the same reason:
no path builds it, and the closure that would is not written. The variant
takes no part in the conversions, as before. A variant with one named
field and no mark is still a variant of another shape.

This makes a variant compile that did not; nothing that compiled breaks.

## Evidence

- `macros/src/subject.rs`, `variant_entry`, which reads the variant's one
  named field as `PayloadField::OnlyNamed`, the struct's case.
- `macros/src/refusal.rs`, `Refusal::NotRebuilt`, whose sentence for a
  variant names the tuple variant.
- `tests/ui/self_in_an_option_over_a_variants_named_field`, the case for
  the sentence; `docs/limits.md`, its line; the golden
  `tests/expand/named_field_variant` and the behaviour test
  `rebuilt_named_field_variants`, the rebuild.

## Not chosen

- Keeping the limit, with a sentence saying that a variant's one named
  field is not rebuilt. True, and a refusal of what a hand writes in one
  line, beside a struct that is rebuilt the same way.
