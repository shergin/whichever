# A mark under `cfg_attr` is a sentence

Status: accepted, 2026-11-17. Serves rules 1 and 4 of the
[vision](../vision.md), the output is the hand's and a limit is one
sentence on the item. Reopen if `#[cfg_eval]` is stabilized, which decides
the `cfg_attr` attributes inside an item before an attribute macro sees
them, or if a surveyed project's target differs between builds and the
item written twice under `#[cfg]` is what it writes by hand.

## Context

An attribute macro is handed its item with the `cfg` and `cfg_attr`
attributes inside it as they are written: rustc decides them afterwards,
and a macro cannot evaluate a predicate. A `#[cfg_attr(feature = "x",
dispatch)]` on a field, a variant or a method was therefore read in no
build. The struct was told to mark the field it had marked, and rustc then
reported `dispatch` as an attribute it does not know. The spelling is the
natural one for a struct whose trait is forwarded in some builds only:
uutils' tee implements `AsFd` for its `NamedWriter` by hand under a `cfg`.

## Decision

A `dispatch` under `cfg_attr` is one sentence on the mark, naming the way
around it: a field is marked in every build, with a bare
`#[whichever::dispatch]` on the struct where its own attributes are under
`cfg_attr`; a variant or a method whose mark differs between builds is
written twice, under `#[cfg(..)]` with the mark and under
`#[cfg(not(..))]` without it. The item is then read as if the mark were
written plainly, and the `cfg_attr` is emitted without it, so the sentence
stands alone.

## Evidence

- Issue #33: what the macro is handed, shown by a macro that prints its
  input, and every place a mark goes with the predicate holding and not.
- `tests/ui/`, one case per place; `tests/behaviour/`, the struct and the
  variant written the way the sentence names, under a predicate that
  holds and under one that does not.

## Not chosen

- Reading the mark as one under the predicate, and writing the impl, the
  arm or the method twice under `#[cfg]`: the builds multiply with every
  such mark, a struct's target would differ per build, and the ways above
  already express every case.
- Documenting it only: the two misleading sentences would stay.
