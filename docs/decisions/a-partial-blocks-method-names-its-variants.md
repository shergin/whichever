# A partial block's method names its variants

Status: accepted, 2026-11-05. Serves rule 3 of the [vision](../vision.md),
what earns a shape, by composition: the inherent method's attribute,
written on a trait method inside the partial block. Reopen if a surveyed
project needs a body per variant rather than one body for the rest, which
is the Swift proposal of 2016 and Zig's `inline` prongs, and a different
shape.

## Context

Trait dispatch forwards every method to every variant, and the partial
block replaces a method for every variant. Nine surveyed projects wrote a
trait whose method some variants answer and a body answers for the rest:
libwifi's `Display` for a `Category` with a unit `Others`, polars'
`Inputs` with `Empty => None`, forest's `DbImpl`, boreholeio, nostralink,
derive-sql's thirty `ToSql` arms behind one `Null`, mlt-core's three
enums, sourmash, and Lemmy's `Page => Err(NotFound)` on a payload that has
an impl. The composition that existed, an inherent method naming the
variants plus a one-line impl calling it, was never shorter than the
`match`. The same nine, with drevo and lsm-tree, wrote the trait's default
by hand where the block could not say "leave it".

## Decision

Inside a `#[dispatch] impl Trait for Enum` block, `#[dispatch(Variant,
..)]` on a method forwards it to the named variants with fully qualified
calls and makes the body the arm for every other variant, two different
variants of a `Self` argument included, so no `mismatch` is needed; `*`
on a bodiless method is the plain forwarding written out, with the
trait's `mismatch` rule; `#[dispatch(default)]` on a bodiless method
leaves it to the trait's default, and is reported when the trait has
none. `via` applies as for an inherent method; `call`, `with`, `field` and
`value` are reported, since a trait method is called by its own name. A
unit or multi-field variant the block's methods never name is no error.

## Evidence

- The planning note `notes/archive/foreign-traits.md`, question 3, and
  the review of 2026-11-05, section 4.5.
- `tests/expand/partial_block_methods.rs`; `tests/behaviour/`,
  `partial_block_methods`, with a foreign `Display` over a unit variant;
  `tests/ui/block_default_without_a_default.rs`,
  `block_method_with_call.rs`, `block_bodiless_without_attribute.rs`.

## Not chosen

- An enum-level `only(..)` option: a second place to say what the method
  already says, and nowhere to put the body.
- A per-variant override written as a second method of the same name:
  not Rust.
