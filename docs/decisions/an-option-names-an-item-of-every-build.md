# An option names an item of every build

Status: accepted, 2026-11-14. Serves rule 3 of the [vision](../vision.md),
a shape earns support when real code has it, and rule 4, a limit is one
sentence with the way around it. Reopen when a surveyed project gives a
`mismatch` on an enum for a trait method under a `#[cfg]`, in a trait it
cannot put the answer on.

## Context

A trait item's `#[cfg]` is decided in the trait's crate: the carrier macro
is written once per combination of the trait's predicates and holds the
items that exist under it
([the record](a-trait-items-cfg-is-decided-in-the-traits-crate.md)). In a
build without a method, the enum's expansion reads a trait that never had
it. `mismatch(same = false)` on the enum, for a `same` the trait declares
under a feature that is off, is then an option naming nothing in the
trait, reported as a misspelling is, and the enum builds only with the
feature on. A hand's impl has the method under the feature with its
fallback arm inside and builds either way. The conformance matrix found
it; no surveyed project writes it. Two directions: let the carrier name
the methods its combination leaves out, so that the option is dropped in
those builds and a misspelling is still caught; or keep the report and
say where such a method takes its answer.

## Decision

The second. An option on the enum names an item the trait has in every
build. A method under a `#[cfg]` takes its mismatch on the trait method,
`#[dispatch(mismatch = ..)]`, which travels with the method and under its
`cfg`; the sentence for an option naming no method says so, and
`docs/limits.md` has the line. Where the trait is another crate's and
cannot be edited, the enum's attribute is written twice under
`cfg_attr`, repeating the predicate where the enum is.

## Evidence

- `tests/ui`, the case for the sentence with its way around.
- The conformance matrix, whose cells with a method compiled out and a
  mismatch on the enum are limits answered by that sentence.

## Not chosen

- A segment of the carrier naming the methods a combination leaves out.
  It is an addition older readers skip, so it could be made; it is a
  change to a surface frozen into every published trait crate, for a
  shape nobody has asked for, and the trait-side spelling already says it
  once for every enum.
