# A borrow a shared pointer cannot lend is a sentence

Status: accepted, 2026-11-14. Serves rule 4 of the [vision](../vision.md),
a limit is one sentence on the item. Reopen if the names `Rc` and `Arc`
turn out to be someone's own types that lend mutably, for which the
sentence would be wrong where rustc would have accepted the arm.

## Context

A method taking `self` by value, forwarded to a `deref` variant over an
`Rc`, an `Arc` or a reference, is one sentence on the payload: nothing can
be moved out of such a pointer. A method taking `&mut self` over an `Rc`,
an `Arc` or a `&T` was rustc's E0596 in the generated arm instead, cannot
borrow data in an `Rc` as mutable. A hand gets the same error, so the
arm was not wrong; the user was told by the compiler, about code they did
not write, what the crate tells in a sentence one receiver over.

## Decision

It is a sentence on the payload: the method borrows its target mutably,
the variant dereferences a pointer that cannot lend that, hold a `Box` or
a `&mut`. The three pointers are recognised as the move's are, `Rc` and
`Arc` by the name the path ends in and `&T` by its type; any other
pointer is left to rustc, which knows whether it implements `DerefMut`.
An arm that is a place is asked what its return type borrows, not what
its receiver is, so `fn name(&mut self) -> &Name` through an `Rc` still
forwards.

## Evidence

- `tests/ui`, the case for the sentence; `docs/limits.md`, its line.
- The conformance matrix, whose cells with a mutable borrow through a
  shared pointer are limits answered by it, where they were rejected
  cells holding rustc's error.

## Not chosen

- Leaving it to rustc. The error is right and, since the bindings are
  located where the arm is, lands on the enum's name; it still speaks of
  an arm the user cannot see, where its sibling speaks of their variant.
