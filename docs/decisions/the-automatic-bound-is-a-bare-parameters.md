# The automatic bound is a bare parameter's

Status: accepted, 2026-11-17. Serves rules 1 and 4 of the
[vision](../vision.md), the output is the hand's and a limit is one
sentence on the item. Reopen if rustc reports an impl's unsatisfiable
`where` clause at the impl rather than at every use, or if a surveyed
project's wrappers need a bound the empty block cannot write.

## Context

A payload that is a type parameter of the enum or the struct, `Raw(T)`, or
`Box<T>` under `deref`, gives the impl the bound `T: Trait`, which the arms
need and a hand writes. A payload that only mentions a parameter,
`Wrapper<T>`, gives none, and the impl does not compile until an empty
partial block writes the header: `impl<T: Shape> Shape for Any<T> {}`.
fvm 4.9.0 has 18 such sites and sux 0.15.4 three. ambassador writes
`where Wrapper<T>: Trait` for every field by itself.

## Decision

The automatic bound stays where it is. The bound a payload like
`Wrapper<T>` needs is whatever its own impl asks of `T`, which is not
written in the enum: `C: CallManager` for fvm's `Kernel<C>`, another trait
than the one forwarded. The one bound the attribute could write without
knowing it is `where Wrapper<T>: Trait`, and that bound was measured:

- An enum whose payload names the enum, through `Box` or by `Self`,
  compiles today; with the bound its impl is satisfied by no type, every
  use is E0599, and a qualified call is E0275, an overflow.
- A payload whose type has no impl for the enum's parameters is E0277 on
  the payload today; with the bound the impl compiles and the error moves
  to every use, as a method the enum lacks.

The empty block is two lines per trait and writes the hand's header, the
bound on another trait included.

## Evidence

- Issue #31: fifteen payload shapes compiled both ways, with what rustc
  says of each.
- `tests/behaviour/`, `bounds_for_type_parameter_payloads`: the empty block
  over a payload that mentions a parameter, for an enum and a struct.

## Not chosen

- `where Payload: Trait` for every payload that mentions a parameter: the
  two regressions above.
- The same, leaving out a payload that names the enum or `Self`: the moved
  errors stay, and a cycle through a payload's own impl is not visible in
  its tokens.
- An option that writes the `where` clause asked for, `where(C:
  CallManager)`: one line where the empty block is two, and a second way
  to write an impl's header.
