# The subject can be restated

Status: accepted, 2026-11-05. Serves rules 1 and 2 of the
[vision](../vision.md), the output is the hand's and nothing between
expansions, and rewords one bullet of its "What it will not be". Reopen if
a surveyed project needs conversions on a restated enum into a local type,
`into = Local`, which the orphan rules allow and this record leaves out.

## Context

The vision had refused dispatch over an enum another crate defines: "the
attribute sits on the definition, as a hand-written impl would have to."
The second clause is not what Rust says: a hand-written `impl LocalTrait
for other::Either<L, R>` is allowed by the orphan rules, and
activitypub_federation and Lemmy write exactly that over `either::Either`
(twelve arms, with a provided `shared_inbox` override hidden), pixi would
over rattler's `ShellEnum`, and `enum_delegate`'s `implement_for` and
ambassador's `delegate_remote` exist for it. The refusal rested on a
premise the compiler does not share.

## Decision

An enum is restated the way a trait is, with the same word: `#[dispatch(
remote = other::Either)]` beside the trait attributes on an enum whose
variants name the real one's. Nothing is emitted for the restatement but
its carrier and the impls, written for the real path, with the patterns
naming the real variants; a variant missing or misspelt fails against the
real enum, as a restated signature fails against the real trait. The
restatement gets no `From`, `TryFrom` or `into`, which would implement a
foreign trait for a foreign type, and no inherent impl, which the language
refuses for a foreign type; both are one sentence. A partial impl block is
written on the real enum, imported under the restatement's name, so that
the block's self type is the type and the carrier is found under the same
name in the macro namespace. The vision's bullet becomes: not a way to
read another crate's enum; a restatement names its variants once.

## Evidence

- The review of 2026-11-05, section 4.7, and the sweep's item 29.
- `tests/expand/restated_enum.rs`; `tests/behaviour/restated_enum.rs`,
  with a partial block; `smoke/enums`' `Either` over `smoke_traits::Either`,
  a plain enum of another crate; `tests/ui/restated_enum_conversions.rs`,
  `restated_enum_inherent_block.rs`.

## Not chosen

- Reading the other crate's enum: nothing stateless can.
- Emitting a `use other::Either;` for the user: an import the user did
  not write, in their module.
- `into = Local` on a restatement: allowed by the orphan rules and left
  out until a project asks.
