# Foreign traits are restated, and the common ones ship

Status: accepted, 2026-11-02 (0.3.0). Serves rules 2 and 3 of the
[vision](../vision.md), nothing between expansions and what earns a shape.
Reopen if
rustc exposes a trait's definition to a proc macro, which would make the
restatement unnecessary. A `std` trait missing from `foreign` is not a
reopening: it is added when a surveyed project forwards it by hand.

## Context

`#[dispatch]` reads a trait from its carrier macro, so a trait from `std` or
from a crate the user does not own had no way in, and the surveys kept
finding the same hand-written impls: `Display` and `Error` on error enums,
`Read` and `Write` on stream enums (memcache, whose `Stream` hid the
payloads' overrides of the provided methods), `Iterator`, `Seek`. Three
directions: a restatement the user writes once; a fixed set of declarations
shipped by the crate; the signatures inline in the dispatch attribute.

## Decision

Both of the first two, as one mechanism. `#[dispatchable(remote = path)]`
restates a trait: nothing is emitted for the restatement but its carrier
macro, and dispatching it implements the trait at `path`, forwarding
exactly the methods restated, provided ones included. `whichever::foreign`
ships nine such restatements, `Display`, `Debug`, `Error`, `PartialEq`,
`Iterator`, `Read`, `Write`, `BufRead` and `Seek`, each forwarding every
provided method a `match` can forward and that is stable on the crate's
minimum Rust version. A `std::`, `core::` or `alloc::` path in `#[dispatch]`
is reported with the `foreign` declaration or the restatement to use.

## Evidence

- The planning note `notes/gaps/foreign-traits.md`: the memcache case and
  the cases from four further pages of surveys.
- serde's `#[serde(remote = "..")]`, the precedent for the word and the
  shape.
- `tests/expand/foreign.rs`; `tests/ui/std_path_not_dispatchable.rs` pins
  the sentence.

## Not chosen

- Signatures inline in the dispatch attribute: repeats at every enum and
  reads poorly.
- A shipped set alone: closed, with no way for a user to add their own
  crate's trait.
- Forwarding adapters that consume `Self` into a wrapper, `Iterator::map`
  or `Read::bytes`: a `match` cannot forward them, and the trait's default
  body does the right thing.
