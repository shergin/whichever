# A struct forwards through a marked field

Status: accepted, 2026-11-02 (0.4.0). Serves rule 3 of the
[vision](../vision.md), what earns a shape. Reopen if a
project needs different traits forwarded to different fields of one struct.

## Context

A dispatch enum is often private behind a public struct that forwards the
same trait by hand, and the hand-written half forwards only the required
methods, so a payload's override of a provided method never reaches the
public type (tor-proto's ten wrappers, fmmap's async newtypes,
rattler_shell, proof-of-sql). Two directions: `#[dispatch(Trait, via =
field)]` on the struct, or `#[dispatch(Trait)]` on the field.

## Decision

`#[dispatch(Trait)]` on the struct, forwarding to its one field, or to the
field marked with a bare `#[dispatch]` when it has several. Every method is
forwarded, provided ones included. `#[dispatch(deref)]` on the field reaches
through a pointer. A method returning `Self` is rebuilt only around a struct
with no other field; otherwise it is an error, and the method goes in a
partial impl block, which works on a struct as on an enum. A struct gets no
`From` impls, and its `Self` arguments need no `mismatch`.

## Evidence

- The planning note `notes/gaps/newtype-delegation.md`: the sixteen hidden
  overrides in fmmap's newtypes, the ten forwarders in tor-proto.
- `tests/expand/newtype.rs`;
  `tests/ui/struct_self_return_other_fields.rs` and
  `tests/ui/struct_without_marked_field.rs` pin the sentences.

## Not chosen

- `via = field` as an option: a second way to say what the marker says.
- The trait named on the field: natural for several traits to several
  fields, which no surveyed project does; the attribute's home stays the
  item that gets the impl, as for an enum.
