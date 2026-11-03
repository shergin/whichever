# `impl Trait` returns: an enum forwards futures, a struct forwards them all

Status: accepted, 2026-11-02. Supersedes
[`impl Trait` returns are rejected, except futures](impl-trait-returns-are-rejected-except-futures.md),
whose reopening condition it met. Serves rules 1 and 3 of the
[vision](../vision.md), the output is the hand's and what earns a shape.
Reopen when a second return-position `impl Trait` shape has a single-block
spelling on an enum that a hand would write, and a surveyed project writes
it by hand.

## Context

A `match` with several arms cannot return `impl Trait`, since every arm
would return a different opaque type; that is why an enum forwards
`impl Future<Output = T>` alone, in one `async` block, and rejects the rest.
A struct's `match` has one arm. rsynko's `SyncCommandViewAlg` forwards
`fn items(&self) -> impl Iterator<Item = u32>` through a newtype by hand,
`self.inner.items()`, which is one arm returning one opaque type, and the
enum's reason does not apply.

## Decision

- On an enum, nothing changes: `-> impl Future<Output = T>`, with `Send` or
  any other bound, is awaited in one `async move` block around the `match`;
  every other return-position `impl Trait`, at any depth of the return type,
  is reported in one sentence on the trait method.
- On a struct, a return-position `impl Trait` is forwarded as it is: the
  one arm returns the field's opaque type, and that is the impl's. A future
  is forwarded the same way, without an `async` block, unless its `Output`
  is a `Self` to rewrap or an overridden associated type to convert, in
  which case the block is where the conversion happens, as on an enum.

## Evidence

- `tests/expand/opaque_through_a_struct.rs`: `impl Iterator` and
  `impl Future<Output = u32> + Send` forwarded in one arm, and
  `impl Future<Output = Self>` awaited and rewrapped; the behaviour test
  `opaque_returns_through_a_struct` runs them.
- `tests/ui/rpitit.rs` still rejects the enum, and `tests/ui/rpitit_nested.rs`
  rejects an `impl Trait` inside a `Result`, which reached rustc before.
- The planning note `notes/bugs/rpitit-rejected-on-a-struct.md`, with the
  rsynko case.

## Not chosen

- Keeping the `async` block for a struct's futures: a future layer a hand
  does not write, which also makes the returned future `!Unpin` where the
  field's may be `Unpin`.
- Boxing or an enum of opaque types on an enum: as in the superseded record.
