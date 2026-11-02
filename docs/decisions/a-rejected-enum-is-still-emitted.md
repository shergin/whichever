# A rejected enum is still emitted

Status: accepted, 2026-11-02 (0.2.2; `#[cfg]` in 0.4.0). Serves rule 4 of
the [vision](../vision.md), a limit is one sentence; this is its mechanism.
Reopen never; superseding it would mean changing the rule.

## Context

When `#[dispatch]` rejected anything on an enum, a variant shape or an
option, it emitted only the `compile_error!` and dropped the item. Every
use of the enum then failed too: E0206 on the derive above it, "cannot find
macro" on its inherent block, E0432 at every `use`. proof-of-sql names its
`DynProofExpr` in 42 files.

## Decision

On error the original enum is emitted with the `#[dispatch]` helper
attributes stripped from its variants, followed by the one `compile_error!`
at the span of the item at fault. Derives and uses resolve, and what is left
is the real error plus, at worst, the missing trait methods. A variant whose
shape is rejected is reported under its own `#[cfg]`, so a variant compiled
out, such as a test-only mock, is no error.

## Evidence

- The proof-of-sql probe in `notes/gaps/diagnostics.md`, section 1, with
  the three cascading errors.
- `tests/ui/rejected_variant_keeps_enum.rs`: a rejected variant on an enum
  with a derive and a use, one error. `tests/ui/cfg_enabled_bad_variant.rs`
  for the `#[cfg]` case.

## Not chosen

- Dropping the item: the cascade above.
- Emitting the enum unchanged, helper attributes included: rustc then
  reports the unknown attributes as a second error.
