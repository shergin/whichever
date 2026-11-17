# Moving from `enum_dispatch`

The swap is mechanical. You change two attributes, give each variant its
payload type in full, and keep everything else.

## Before and after

With `enum_dispatch` 0.3:

```rust
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
}

#[enum_dispatch(Shape)]
pub enum AnyShape {
    Circle,
    Square,
}
```

With `whichever`:

```rust
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
}

#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

The trait, its implementors and the code that calls it stay as they
are. A `Circle` still converts into an `AnyShape` with `.into()`, and
`shape.area()` still forwards to the variant inside.

## The steps

1. Run `cargo add whichever` in the crate that owns the trait and in the
   crate that owns the enum.
2. On the trait, replace `#[enum_dispatch]` with
   `#[whichever::dispatchable]`. If the trait lists its enums, as in
   `#[enum_dispatch(AnyShape)]`, drop the list: the enum names the trait
   instead.
3. On the enum, replace `#[enum_dispatch(Trait)]` with
   `#[whichever::dispatch(Trait)]`. An enum that lists several traits
   gets one attribute per trait.
4. Write each variant with its payload: `Circle` becomes
   `Circle(Circle)`.
5. Build, and fix what the three things to watch below point at: most
   often a missing `use` at the enum.
6. Remove `enum_dispatch` from `Cargo.toml` once nothing uses it.

## The mapping

| `enum_dispatch` | `whichever` |
| --- | --- |
| `#[enum_dispatch]` on the trait | `#[whichever::dispatchable]` |
| `#[enum_dispatch(Trait)]` on the enum | `#[whichever::dispatch(Trait)]` |
| `#[enum_dispatch(A, B)]` | `#[whichever::dispatch(A)]` and `#[whichever::dispatch(B)]` |
| `#[enum_dispatch(Enum)]` on the trait | name the trait on the enum instead |
| `enum E { Circle, Square }` | `enum E { Circle(Circle), Square(Square) }` |
| `From<Payload>` | the same, opt out with `from = false` |
| `TryInto<Payload>`, error `&'static str` | `try_from = true`: `TryFrom<E> for Payload`, the error hands the enum back; `ref` and `ref_mut` for the by-reference forms |

The `from` and `try_from` options are described in
[options.md](options.md).

## Three things to watch

- **The types in the trait's signatures must be in scope where the enum
  is**, as for a hand-written impl, because the impl is written next to
  the enum. `enum_dispatch` sometimes emitted the impl next to the trait
  instead. So expect a few new `use` lines at the enum, and a few unused
  ones at the trait.
- **Attribute order matters with `#[async_trait]` and
  `#[typetag::serde]`.** Put `#[whichever::dispatchable]` below
  `#[async_trait]`, so that `whichever` reads the signatures after
  `#[async_trait]` has rewritten them. Put it above `#[typetag::serde]`,
  because `typetag` adds its own per-impl methods to the trait, and a
  forwarded one would return the payload's tag. Either order the other
  way round gives you an error that says where to put the attribute.
- **A generic trait lists its parameters on the enum**:
  `#[whichever::dispatch(Visitor<'a>)]`.

## What you gain

Once the swap is done, the enum can do what `enum_dispatch` cannot:

- the trait in one crate and the enum in another,
- associated types and consts,
- `&Self` arguments, and `Option<Self>` and `Result<Self, E>` returns,
- traits you do not own, such as `Display`.

[The comparison](comparison.md) has the full table and an example of a
trait `enum_dispatch` cannot take.
