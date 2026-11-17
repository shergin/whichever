# A required method's `#[inline]` is its forwarders'

Status: accepted, 2026-11-17. Serves rules 1 and 2 of the
[vision](../vision.md), the output is the hand's and nothing between
expansions. Reopen if a surveyed project has to choose the inlining at the
enum or the struct, for a trait whose author marked nothing, and the
partial block that says it per method is what it writes by hand.

## Context

A forwarder carries the trait method's own `#[inline(..)]`, else
`#[inline]`. That reached a required method only through an attribute
rustc is taking away: `#[inline]` on a trait method without a body is a
warning on stable 1.100, "the `inline` attribute cannot be used on required
trait methods", announced as a future hard error, and Clippy denies it by
default. sux 0.15.4 asks for `#[inline(always)]` on every forwarder of 28
traits, most of whose methods are required.

## Decision

`#[dispatchable]` reads an `#[inline(..)]` on a method without a body as
what it can only mean, the inlining of that method's forwarders: the
attribute travels in the carrier macro, as it did, and stays off the
trait's own method, where there is no code to inline. The trait compiles
without a warning under rustc and Clippy, on 1.88 as on stable, and the
forwarder carries the attribute. A provided method keeps its attribute,
which is its own body's as well.

Nothing changes in what the carrier holds, so a trait published with this
release is read by an older one as before.

## Evidence

- Issue #30, with rustc's and Clippy's messages on 1.100 and on 1.88.
- `tests/expand/carried_attributes.rs`: the trait without the attribute
  and the forwarders with it; `tests/behaviour/`, `carried_items`.

## Not chosen

- `#[dispatchable(inline = always)]`, for every method of the trait:
  ambassador's spelling, and one word per trait where this is one line per
  method. The line per method is what a hand writes on its own impl, and
  what the trait already says for a provided method.
- `inline = always` on the enum's or the struct's attribute: a second
  place that says how a forwarder is inlined, written at every dispatch of
  the trait.
- A marker of this crate's own, `#[dispatch(inline = always)]`: a second
  spelling of an attribute Rust has.
- Carrying an `#[inline(..)]` written under `cfg_attr`: its predicate is
  the trait's crate's to decide, and the forwarder is written in another.
