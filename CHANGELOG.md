# Changelog

## 0.2.2 - Unreleased

- The hidden macro that `#[dispatchable]` exports is named after the
  position of every token of the trait as well as its text, so one
  `macro_rules!` may emit the same trait in several modules when the
  invocation supplies the trait's name or any other token. Before, both
  expansions got the same name and rustc reported it as defined multiple
  times. A trait written out in full in the macro's body still collides;
  the limit is documented.

## 0.2.1 - 2026-11-02

- A generic enum whose payloads name its type parameters can dispatch a
  trait with an associated type that is not overridden. The agreement check
  used a nested function, which cannot name the enum's generics.
- Receivers written as `self: Self`, `self: &Self`, `self: &'a Self` and
  `self: &mut Self` dispatch like the shorthand forms; any other receiver
  type is reported as unsupported.

## 0.2.0 - 2026-11-02

- `#[dispatch]` on an inherent `impl` block forwards each method marked
  `#[dispatch(Variant, ..)]` to those variants' own method, with the body as
  the arm for every other variant; `*` names every variant and the method
  is then written without a body; `with = path` wraps the result and
  `call = name` renames the target. A bare `#[dispatch]` on an enum
  registers it for this without dispatching a trait.
- `try_from = true` adds `TryFrom<Enum> for Payload`, returning the enum
  when it holds another variant.
- The generated `match` uses the receiver's own `self` token, so methods
  written in an impl block resolve it correctly.

## 0.1.0 - 2026-11-01

Initial release.

- `#[dispatchable]` on a trait and `#[dispatch(Trait)]` on an enum, across
  crates, with no state kept between expansions.
- Associated types from the first variant, checked against every other
  variant, or fixed with `Name = Type` and unified through `Into`.
- Associated consts from the first variant.
- `Self`, `&Self` and `&mut Self` arguments matched on the same variant,
  with the fallback declared by `mismatch`.
- `Self`, `Option<Self>` and `Result<Self, E>` returns wrapped back into
  the variant.
- Generic traits at a concrete instantiation, including lifetimes, const
  generics and `W::Assoc` projections; generic enums.
- `async fn`, `unsafe fn`, generic methods, `#[cfg]` on methods and
  variants, `#[inline]` on every generated method.
- `From<Payload>` for every payload type held by one variant, `from = false`
  to opt out.
- `#[dispatch(deref)]` on a variant for `Box<T>`, `Rc<T>`, `Arc<T>`, `&T`
  and `&mut T` payloads.
