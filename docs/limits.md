# Limits

- Every variant a trait is dispatched to holds exactly one unnamed field,
  `Name(Payload)`, or marks the field to forward to with `#[dispatch]`,
  and every target implements the trait. A variant of another shape is
  reported under its own `#[cfg]`, so a variant compiled out, such as a
  test-only mock, is no error; an enum that only forwards inherent methods
  may hold such variants, which take the method's body.
- An enum cannot dispatch a return-position `impl Trait`, at any depth of
  the return type, except `impl Future<Output = T>`, which is awaited like
  an `async fn`; a struct forwards any.
- A variant reached through a route, `deref` included, cannot take part in
  a method returning `Self`: there is no way to put the result back behind
  the pointer or around the other fields. A route ending in a method call
  cannot serve a `&mut self` method.
- A method that requires `Self: Sized` cannot be forwarded to a `deref`
  variant holding a trait object; a partial block leaves it to the trait's
  default with `#[dispatch(default)]`.
- A method whose return type wraps `Self` in anything but `Option`,
  `Result` or a tuple, such as a combinator
  `fn buffered(self) -> Buffered<Self>`, cannot be forwarded: the payload's
  `Buffered<Payload>` is not the enum's. A provided one is left to the
  trait's default in a partial block with `#[dispatch(default)]`, and the
  default then wraps the enum.
- A generic associated type is declared by the enum, in a partial impl
  block; it cannot come from the first variant.
- A projection through a trait parameter with several bounds, `F::Value`
  with `F: Format + Clone`, cannot be qualified at the enum; the trait
  writes `<F as Format>::Value`.
- An associated type or const a partial impl block declares under `#[cfg]`
  is an error: the forwarded methods convert into the type in every build.
  Put the whole block under the `cfg`, with a second block for the other
  builds.
- A lint attribute on the enum does not reach the impls written next to it,
  as it would not reach a hand-written one; `#[allow]` goes on the trait
  method, on a partial impl block, or on the module.
- A private glob import that brings the trait's own name back into the
  trait's module, `use crate::prelude::*;` at a crate root whose prelude
  re-exports a trait declared there, makes the carrier's re-export
  ambiguous to rustc (`ambiguous_import_visibilities`, an error under
  `#![deny(future_incompatible)]`). Declare the trait in a module of its own
  and re-export it, or import the prelude's items by name there.
- Receivers are `self`, `&self`, `&mut self`, `self: Box<Self>` on `deref`
  variants holding a `Box`, and `self: Pin<&Self>` or `self: Pin<&mut Self>`
  when every payload is `Unpin`. `Rc<Self>`, `Arc<Self>` and
  `Pin<Box<Self>>` would need a refcount or an allocation, so they are not.
- A value cannot be moved out of a `deref` variant that holds anything but
  a `Box`: a method taking `self` is reported on the payload, and so is an
  inherent method's `value` or `field` under `self: Box<Self>` unless the
  return type borrows the place. Hold a `Box`, or give the method a body,
  or implement a trait method by hand.
- The types in the trait's signatures are written into the enum's module
  and must be in scope there, as for a hand-written impl; `crate::` paths
  are rewritten to the trait's crate, `super::` and `self::` paths are not.
  They must also be nameable from there: a private alias or type in a
  signature limits the trait to its own crate.
- An associated const of a type other than an integer, a float, `bool`, `char` or
  `&str`, or on a generic enum, is taken from the first variant without a
  check.
- A trait whose items carry more than four distinct `#[cfg]` predicates
  is reported on the fifth: the carrier macro is written once per
  combination of them. Combine the predicates, or move the items under
  the fifth to a trait of their own.
- One `macro_rules!` may emit a `#[dispatchable]` trait in several modules
  when at least one token of the trait, its name or a method's, comes from
  the invocation. A trait written out in full in the macro's body gets the
  same carrier macro name at every invocation, and rustc reports it as
  defined multiple times, unless each invocation passes its own
  `#[dispatchable(name = ..)]`.

Every unsupported shape is reported as one sentence on the offending trait
item or variant; see `tests/ui` for all of them. A rejected enum is still
emitted, so the derives, uses and impl blocks around it compile, and a
trait method that cannot be forwarded is still written, with its sentence
as its body, so the impl is complete and the calls around it resolve: that
one error stands alone.

"cannot find macro `Shape` in this scope" at the enum means the trait is not
marked `#[dispatchable]`, or the trait, which carries the macro, is not
imported where the enum is. A `std::`, `core::` or `alloc::` path in
`#[dispatch]` is reported directly, with the `whichever::foreign`
declaration or the `remote` restatement to use instead. "could not find
`Trait` in `other`" on `#[dispatch(other::Trait)]` means the same for a
trait of another crate: restate it with `remote`. The same words on a
partial block over another crate's enum mean the enum has no
`#[whichever::dispatch]`, which only its own crate can add, or that the
enum is another crate's and is restated with `remote`.

One `macro_rules!` that emits the same `pub` trait, written out in full,
at two places gets the same carrier macro name twice; `#[dispatchable(name =
tokio)]` names the macro explicitly and tells the two apart.
