# A named field is not rebuilt under a wrapper

Status: accepted, 2026-11-14. Serves rule 1 of the [vision](../vision.md),
the output is the hand's and holds no closure. Reopen if a surveyed
project forwards `Option<Self>` or `Result<Self, E>` through a struct
with one named field and cannot make the field unnamed, or if stable Rust
gains a path that builds a struct with named fields.

## Context

A dispatch struct with no other field rebuilds `Self` around what its
field returns. For `-> Self` that is the constructor around the call. For
`-> Option<Self>` and `-> Result<Self, E>` it is `.map(..)`, which takes a
path: `.map(Tuple)` for `struct Tuple(Square);`. A struct with a named
field, `struct Named { shape: Square }`, has no such path, and since
structs shipped in 0.4.0 the attribute wrote `.map(|value| Named { shape:
value })` there. It was the one closure in the generated code, against the
vision's first rule, the README and `docs/what-is-generated.md`, which
all say there is none, and against the rule the engine already kept
elsewhere: a tuple under an `Option` is a limit because its rebuild would
be a closure, "which a hand would write and the attribute does not". The
conformance matrix had no struct with a named field; the closure was found
by reading. Two directions: say in the documents that this one closure is
written, or make the shape a limit.

## Decision

The second. A method that returns `Self` inside an `Option` or a `Result`
is not forwarded through a struct whose one field is named: it is reported
on the field in one sentence, with the tuple struct as the way around it.
`-> Self` on the same struct is rebuilt by its braces as before, and a
tuple struct forwards all three. The rule stays as written, with no
exception to remember, and the tuple under an `Option` and the named
field under one are refused for the same reason.

This breaks a struct that compiled: one named field, and a dispatched
method returning `Option<Self>` or `Result<Self, E>`. No surveyed project
has it; the changelog lists the break.

## Evidence

- `tests/ui`, the case for the sentence; `docs/limits.md`, its line.
- `macros/src/shape.rs`, `check_expressible`, which refuses a tuple under
  a wrapper for the same reason.
- The conformance matrix's shape check, which calls any closure in an
  expansion a violation, and its struct with a named field.

## Not chosen

- Writing the closure and saying so. The generated code would hold what
  the rule says it never holds, and the next shape with no path would ask
  for the same exception.
- A `match` in place of `map`, `match call { Some(value) => Some(Named {
  shape: value }), None => None }`. No closure, and no hand writes it.
