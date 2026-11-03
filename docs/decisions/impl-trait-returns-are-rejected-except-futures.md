# `impl Trait` returns are rejected, except futures

Status: superseded on 2026-11-02 by
[`impl Trait` returns: an enum forwards futures, a struct forwards them all](impl-trait-returns-are-forwarded-through-a-struct.md),
after rsynko met the reopening condition with a struct. Accepted
2026-11-02 (0.2.2). Served rules 1 and 3 of the [vision](../vision.md), the
output is the hand's and what earns a shape.

## Context

A `match` cannot return `impl Trait`: every arm would return a different
opaque type. `async fn` in a trait already dispatched, with each arm
awaited. fmmap 0.5.0 writes its async traits as `-> impl Future<Output = T>
+ Send`, because `async fn` in a public trait cannot promise `Send`, and
forwards 29 such methods by hand per runtime, 87 arms.

## Decision

`-> impl Future<Output = T>`, with `Send` or any other bound, dispatches:
the whole `match` is written inside one `async move` block and every arm is
awaited, so the method returns one future whose bounds, `Send` included,
rustc checks against that block. The same works for inherent methods. Every
other return-position `impl Trait` is rejected, with a sentence that names
the exception.

## Evidence

- The planning note `notes/gaps/async-send-returns.md`: fmmap's methods and
  the expansion, which is what a hand writes for an `async fn` with a
  `Send` bound.
- `tests/expand/async_returns.rs`; `tests/ui/rpitit.rs` and
  `tests/ui/impl_into_future.rs` pin the sentences.

## Not chosen

- Boxing the return, `Box<dyn Trait>`: an allocation the signature does not
  ask for.
- An enum of the arms' opaque types: a helper type a hand would not write,
  and one more thing to name.
- Rejecting futures with the rest: the one `impl Trait` a hand spells in
  one block, and the one the surveys found.
