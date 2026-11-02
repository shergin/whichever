# Changelog

## 0.3.0 - 2026-11-02

- `#[dispatch]` on a trait impl block of a dispatch enum makes it a partial
  impl: the block holds the items the enum implements itself, a method
  without a receiver among them, and every other item of the trait is
  forwarded around them. The block's header carries into the impl, its
  options go on the block's attribute, and an associated type written in
  the block is an override. The enum then leaves that trait out of its own
  attribute.
- `#[dispatchable(remote = path)]` restates a trait of another crate:
  nothing is emitted for the restatement but the hidden macro, and
  dispatching it implements the trait at `path`, forwarding exactly the
  methods restated. `whichever::foreign` ships such declarations for
  `Display`, `Debug`, `Error`, `PartialEq`, `Iterator`, `Read`, `Write`,
  `BufRead` and `Seek`, each forwarding every provided method a `match` can
  forward. A `std::`, `core::` or `alloc::` path in `#[dispatch]` is now
  reported with the declaration or the restatement to use.
- `#[dispatchable(name = ident)]` names the hidden macro explicitly, for a
  `pub` trait that one `macro_rules!` emits several times with identical
  tokens.
- `#[dispatch(from = false)]` on a variant leaves it out of the `From` and
  `TryFrom` impls.
- `#[dispatchable]` with an argument that is not an option is reported as
  such; before, every argument was rejected.
- An associated const of an integer, `bool`, `char` or `&str` type is
  checked at compile time to agree between the variants of a non-generic
  enum, with the error on the variant that differs; before, the first
  variant's value was taken silently. `Name = value` on the enum fixes a
  const that differs per payload by design.

## 0.2.2 - 2026-11-02

- A trait method returning `impl Future<Output = T>`, with `Send` or any
  other bound, dispatches: the `match` is written in one `async move` block
  with every arm awaited, so the enum's future is `Send` whenever the
  payloads' are. The same works for inherent methods. Other return-position
  `impl Trait` is still rejected, now with a message that names the
  exception.
- A trait of any visibility other than `pub` no longer puts an exported
  macro at the crate root. Its carrier macro stays in the trait's module and
  is re-exported with the trait's visibility, which is where a `pub(crate)`
  or private trait can be dispatched from anyway.
- The hidden macro that `#[dispatchable]` exports is named after the
  position of every token of the trait as well as its text, so one
  `macro_rules!` may emit the same trait in several modules when the
  invocation supplies the trait's name or any other token. Before, both
  expansions got the same name and rustc reported it as defined multiple
  times. The position includes the last two components of the file's path,
  so two files with the invocation at the same line do not collide either.
  A trait written out in full in the macro's body still collides; the limit
  is documented.
- `Option<Self>` and `Result<Self, E>` returns are recognised under any
  path, `io::Result<Self>` and `core::option::Option<Self>` included. A
  `Self` anywhere else in a return type, such as in a user alias, is
  reported in one sentence on the return type instead of rustc's type
  mismatch. Inherent methods returning `Self`, `Option<Self>` or
  `Result<Self, E>` wrap the payload's result back into the variant.
- A rejected enum is still emitted, with the macro its impl blocks use, so
  derives, `use` lines and inherent blocks compile and the one real error
  stands alone. An inherent block keeps the methods that can be forwarded
  when one of them cannot.
- rustc notes such as "private method defined here" point at the generated
  method instead of the enum's attribute.

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
