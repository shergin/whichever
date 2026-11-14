# Moving from `enum_dispatch`

The swap is mechanical:

| `enum_dispatch` | `whichever` |
| --- | --- |
| `#[enum_dispatch]` on the trait | `#[whichever::dispatchable]` |
| `#[enum_dispatch(Trait)]` on the enum | `#[whichever::dispatch(Trait)]` |
| `#[enum_dispatch(A, B)]` | `#[whichever::dispatch(A)]` and `#[whichever::dispatch(B)]` |
| `#[enum_dispatch(Enum)]` on the trait | name the trait on the enum instead |
| `enum E { Circle, Square }` | `enum E { Circle(Circle), Square(Square) }` |
| `From<Payload>` | the same, opt out with `from = false` |
| `TryInto<Payload>`, error `&'static str` | `try_from = true`: `TryFrom<E> for Payload`, the error hands the enum back; `ref` and `ref_mut` for the by-reference forms |

Three things to watch:

- The types in the trait's signatures must be in scope where the enum is,
  as for a hand-written impl. `enum_dispatch` sometimes emitted the impl
  next to the trait instead, so expect a few new `use` lines at the enum,
  and a few unused ones at the trait.
- With `#[async_trait]`, put `#[whichever::dispatchable]` below it; with
  `#[typetag::serde]`, above it. Either order the other way round is
  reported.
- A generic trait names its parameters: `#[whichever::dispatch(Visitor<'a>)]`.
