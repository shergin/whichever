# Associated consts are checked for four types

Status: accepted, 2026-11-02 (0.3.0). Serves rule 4 of the
[vision](../vision.md), a limit is one sentence, whose opposite failure is
silence. Reopen when const evaluation can compare a
value of any type, for example through const trait methods; the check then
extends to every type and its line in [limits](../limits.md) goes.

## Context

0.1.0 took every associated const from the first variant with no check,
unlike associated types. celestia-types 1.3.0's `FraudProof::TYPE` is a
discriminator that differs per payload by design; dispatching it over a
two-variant enum would compile and answer the first variant's value for
both. Three directions: require an explicit value whenever there is more
than one variant; check agreement at compile time where the type allows;
leave the rule and warn in the docs.

## Decision

An associated const of an integer, a float, `bool`, `char` or `&str` type, on a
non-generic enum, is checked at compile time to agree between the variants,
with the error on the variant that differs. `Name = value` on the enum fixes
a const that differs per payload by design, as `Name = Type` fixes an
associated type. A const of any other type, or on a generic enum, is taken
from the first variant without a check, and [limits](../limits.md) lists
that as a limit.

## Evidence

- A `const` block compares integers, floats, `bool` and `char` directly and
  `&str` through a `const fn` byte loop; floats joined on 2026-11-06, since
  their comparison in a `const` is stable from Rust 1.82, within the
  crate's minimum; the macro cannot see a type's kind beyond
  its tokens, so the recognised set is syntactic.
- The planning note `notes/gaps/diagnostics.md`, section 3, with the
  celestia case.
- `tests/ui/associated_const_disagreement.rs` pins the sentence;
  `tests/expand/associated_consts.rs` holds the override.

## Not chosen

- Requiring an explicit value whenever there is more than one variant:
  breaks every enum whose consts agree, which is most of them.
- Leaving the first-variant rule with a warning in the docs: silence where a
  check is possible.
