# A restated struct lists the fields it forwards to

Status: accepted, 2026-11-17. Completes
[The subject can be restated](the-subject-can-be-restated.md), which speaks
of an enum alone. Serves rules 1 and 4 of the [vision](../vision.md), the
output is the hand's and a limit is one sentence on the item. Reopen if a
surveyed project restates a struct and a stale restatement costs it a bug
that a full list of fields would have caught.

## Context

`remote = path` on a struct's attribute restates another crate's struct as
it restates an enum, and the impl is written for the real path. That
worked, and nothing said so: no document, no test, and three sentences a
restated struct can reach called it an enum.

A restatement that left a field out compiled or not by how many it listed.
With two of three fields the pattern ended in `..`; with one of two it was
the pattern of a struct's one field, which has none, and rustc reported
"pattern requires `..` due to inaccessible fields" with a suggestion that
is not Rust.

## Decision

A struct is restated as an enum is, and the documents, the tests and the
sentences say so in the struct's words.

A restated struct lists the fields its impls forward to, and the real
struct may have more: its pattern always ends in `..`, as a hand's does
over a struct it does not own, `other::Pair { left: inner, .. }`. A field
misspelt, or of another type, still fails against the real struct, and so
does `-> Self` built around a struct that has more fields than the one
listed. An enum's restatement lists every variant, since its `match` has
to.

## Evidence

- Issue #35, with a restated struct compiled against a second crate.
- `tests/expand/` and `tests/behaviour/`, a restated struct with a marked
  field, a field that names a trait, `-> Self` and a partial block;
  `tests/ui/` pins the three sentences.

## Not chosen

- Refusing `remote` on a struct: a hand may write `impl LocalTrait for
  other::Pair`, so the refusal would be narrower than the hand.
- A pattern that never ends in `..`, so that a field left out always
  fails: the forwarding is right without the other fields, and the message
  would still be rustc's about a pattern the user did not write.
