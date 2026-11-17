# `From` reaches one level

Status: accepted, 2026-11-17. Completes
[`From` is opt-out and `TryFrom` is opt-in](from-is-opt-out-and-try-from-is-opt-in.md),
which says nothing of an enum held by another. Serves rule 3 of the
[vision](../vision.md), what earns a shape. Reopen if a surveyed project
holds two dispatch enums in one and writes the impls for the second by
hand, where one blanket impl does not reach.

## Context

An enum whose variant holds another dispatch enum gets `From<Inner> for
Outer`, and nothing for the inner enum's payloads: a leaf converts in two
steps, `Outer::from(Inner::from(leaf))`. typedown-lang 0.63.1 writes one
`From<Leaf> for Outer` per variant of its inner enum by hand, 27 of them
through a `macro_rules!` of its own, and the list grows with every variant.

The attribute cannot write those impls: a payload is a type to it, and
what the inner enum holds travels in the inner enum's own carrier macro,
which never leaves its crate.

## Decision

Nothing is added. The conversions stay one impl per payload type, and a
hand writes the second level once, whatever the inner enum holds:

```rust
#[whichever::dispatch(Shape)]
pub enum Outer {
    #[dispatch(from = false)]
    Inner(Inner),
    Dot(Dot),
}

impl<T> From<T> for Outer
where
    Inner: From<T>,
{
    fn from(value: T) -> Self {
        Outer::Inner(Inner::from(value))
    }
}
```

`from = false` leaves the variant's own `From<Inner>` out, which the
blanket impl covers through `From<T> for T`. The impl does not grow with
the inner enum, and reaches a third level through the second's own.

## Evidence

- Issue #28: the blanket impl compiled beside the generated `From` impls,
  with the inner enum in the same crate and in another, and the three
  places it stops: the variant's `From` left on, a second inner enum, and
  an inner enum and a direct payload that are both another crate's.
- `tests/behaviour/`, `from_impls`: a leaf converted through two and
  through three levels.

## Not chosen

- An option that asks the inner enum's carrier for one impl per leaf, the
  hand's output exactly: a new message between the macros, an inner enum
  of another crate out of reach, and five lines saved once.
- The blanket impl written by the attribute for a variant that asks: an
  option whose whole effect is those five lines, with the same three
  limits to explain.
- A list of the leaves on the variant, `from(Leaf, ..)`: typedown-lang's
  list under another name, which still grows with every variant.
