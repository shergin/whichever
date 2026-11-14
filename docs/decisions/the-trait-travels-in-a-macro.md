# The trait travels in a macro, not a cache

Status: accepted, 2026-11-01 (0.1.0). Serves rule 2 of the
[vision](../vision.md), nothing between expansions. Reopen if rustc gives a proc macro a sanctioned way to read another item's
definition; the carrier macro would then be an implementation detail that
could change, and the rule would not.

## Context

A proc macro sees one item. Writing `impl Trait for Enum` needs the trait's
signatures at the enum, and the trait may be in another crate. Three ways
were in the field: remember the trait in the proc-macro server's process
(`enum_dispatch`), make the user list the signatures again at the enum
(`delegate`), or have the trait's attribute emit a `macro_rules!` that the
enum's attribute invokes (`ambassador`, `portrait`).

## Decision

`#[dispatchable]` emits the trait unchanged plus a carrier macro, a
`#[doc(hidden)]` `macro_rules!` holding the trait's tokens, re-exported
under the trait's own name. `#[dispatch(Trait)]` invokes it, and the carrier
hands the trait's tokens, the enum's tokens and the options to the
implementation macro, which writes the impl in one expansion and keeps
nothing. The spans survive, so errors point at the user's tokens.

## Evidence

- `enum_dispatch`'s cache cannot see a trait from another crate: the enum
  compiles with no impl and the first call fails; it panics on associated
  types and on methods without a receiver; its errors lose their spans
  ([the comparison](../comparison.md), and its table's first row).
- The smoke crates: `smoke/traits` declares, `smoke/enums` dispatches, in a
  second crate, in CI since 0.1.0.
- Under rust-analyzer, which expands in its own process, the attributes
  work because there is nothing to share.

## Not chosen

- A process-global cache: the three failures above are its consequences,
  not bugs in it.
- Re-listing the signatures at the enum: the hand-written impl with a
  different syntax.
- A macro named after the trait at the crate root, as `ambassador` exports
  one: two traits of one name collide, and the macro is a second thing to
  import. The carrier is re-exported under the trait's own name and named
  by its tokens and their position
  ([the record](the-carrier-macros-name-and-home.md)).
