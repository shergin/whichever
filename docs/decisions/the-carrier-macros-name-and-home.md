# The carrier macro's name and home

Status: accepted, 2026-11-02 (0.2.2 and 0.3.0). Serves rule 2 of the
[vision](../vision.md), nothing between expansions. Reopen if a `pub` trait inside a private module, a sealed trait, collides in
a real project, or if rustc offers an export that two identical traits
cannot share.

## Context

In 0.1.0 every dispatchable trait's carrier macro was exported at the crate
root, named after the trait's text and the line and column of the
attribute. The surveys found five costs: a `pub(crate)` trait's macro
reached the crate root anyway (cardinality-estimator, tor-proto); the name
changed when code above the trait moved; a glob re-export carried the macro
into downstream crates (rspotify's prelude); one `macro_rules!` emitting the
same trait twice from two files produced the same name, E0428 (fmmap); and
two carriers could not share one trait, E0252.

## Decision

- A trait of any visibility other than `pub` keeps its carrier macro in its
  own module, re-exported with the trait's visibility. Nothing reaches the
  crate root for it.
- A `pub` trait's carrier macro is named after the text of every token of
  the trait and the position of each, with the last two components of the
  file's path, so the name is the same on every machine and differs
  between two emissions of one `macro_rules!` body whenever any token comes
  from the invocation.
- `#[dispatchable(name = ident)]` names the macro explicitly, for the one
  case the position cannot tell apart: a trait written out in full inside a
  `macro_rules!` body and emitted twice.

## Evidence

- The five cases above, in the planning note `notes/gaps/carrier-macro.md`,
  with the project and the file for each.
- `proc_macro::Span::file()` and `line()` are stable since 1.88, the
  crate's MSRV.
- The 0.2.2 and 0.3.0 release entries; the smoke crates, which import a
  trait from another crate and dispatch it, still pass.

## Not chosen

- A random name, as `enum_delegate` uses: not reproducible across builds.
- A stable name from the tokens alone, without position: two identical
  traits in two modules would share it, and telling them apart would need
  knowledge of the other, which is state between expansions. Dropped.
- Catching E0428 and explaining it: a proc macro cannot see rustc's later
  error; the `name =` option is documented in the README instead.
