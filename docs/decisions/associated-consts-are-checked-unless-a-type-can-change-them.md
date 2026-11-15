# Associated consts are checked unless a type can change them

Status: accepted, 2026-11-14, superseding [Associated consts are checked
for four types](associated-consts-are-checked-for-four-types.md), whose
types it keeps. Serves rule 4 of the [vision](../vision.md), a limit is one
sentence, whose opposite failure is silence. Reopen when const evaluation
can compare a value of any type, or check a const that a type parameter
decides; or if a payload is found that implements the trait for some
lifetime other than `'static` and not for `'static`, for which the check
would reject an enum that forwards.

## Context

The record this one supersedes checked a const of an integer, a float,
`bool`, `char` or `&str` on a non-generic enum, and took it from the first
variant without a check on any generic one. An outside review of the
conformance matrix's model showed what that costs on an enum generic over
a lifetime alone: `enum E<'a> { A(P0), #[dispatch(deref)] B(&'a P1) }`, with
`LEVEL` 7 on `P0` and 8 on `P1`, compiled and answered 7 for both, where the
same enum without the lifetime is an error on `B`. The reason for the
exception is that the check is a free `const` item, evaluated where it is
written, and a const that a type or const parameter decides cannot be
evaluated there; a check inside a generic item is never evaluated at all.
No lifetime decides a const's value.

## Decision

The check runs on every enum without a type or const parameter. Its
lifetime parameters, where it has any, are `'static` in the check: the
payloads' consts are compared as `<&'static P1 as Trait>::LEVEL`, which is
the value every lifetime gives. A variant reached through a field or a
method is still left out, its target having no type to name there, and
`docs/limits.md` says so; so is an enum with a type or const parameter.

This breaks an enum that compiled: one with lifetime parameters alone whose
variants disagree on such a const, which answered the first variant's value
and now is an error on the variant that differs. The changelog lists it.

## Evidence

- The review's probe above, and `tests/ui/const_disagreement_on_a_lifetime_enum`,
  which pins the error on the variant.
- `tests/behaviour/associated_const_agreement`, where an enum with a lifetime
  and agreeing consts compiles and answers the value.
- A trait implemented for every lifetime of a payload is implemented for
  `'static`; an enum whose forwarding compiles for every `'a` names payloads
  that implement the trait at each, so the check asks nothing the impl does
  not.

## Not chosen

- The check inside an impl generic over the enum's lifetimes, as the type
  check's helpers are: an associated const of a generic impl is evaluated
  only where it is used, so the check would never run.
- Keeping the exception and saying so: silence where a check is possible,
  which the superseded record already refused.
