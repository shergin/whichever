# Reaching the target

An arm calls the trait on the payload. A **route** names the steps from
the payload to the value the trait is called on instead: `*` for what the
payload dereferences to, which `deref` spells, `.field` or `.0` for a
field, `.method()` for what a method returns, in any sequence. A route is
written in four places, which are the places a hand writes it:

- on a variant or a struct field, for every trait: `#[dispatch(via =
  .base_account)]`, `#[dispatch(via = .1)]` on `VarChar((&[&str], &[S]))`;
- as the marker a struct already uses, on one field of a variant with
  several: `ConOut(#[dispatch] LineWriter<File>, u32)` or `Node { id: u32,
  #[dispatch] data: Data }`; the other fields ride along in the pattern;
- on the trait, keyed by variant, when the route differs per trait:
  `#[dispatch(ProvableResultColumn, via(VarChar = .0))]` beside
  `#[dispatch(MultilinearExtension, via(VarChar = .1))]`;
- on an inherent method, for that method alone: `#[dispatch(*, via =
  .content(), call = refresh)]`.

The arm stays one line. Where the route names the target's type, the
payload or what it dereferences to, the call is `<Square as Shape>::area(
&inner.base)`; a field's or a method's type cannot be named, and the call
is `<_ as Shape>::area(&inner.base)`, qualified by the trait alone, so an
inherent method of the same name still never takes over. A route ending
in a method call yields a value: it serves `&self` and `self` methods, and
a `&mut self` method is reported, since nothing can be borrowed mutably
through a call. A method returning `Self` is reported on a routed variant,
`deref` included, since the rest of the payload is gone. A variant with
other fields beside the payload gets no `From` or `TryFrom`, and a variant
whose target has no nameable type takes no part in the agreement checks,
where a disagreement shows in the arm instead.

A `deref` variant holding a trait object, `Box<dyn Trait>` or `&dyn Trait`,
is how an enum holds an open set beside its sized payloads. A method the
trait declares with `where Self: Sized` cannot be called on it, and is
reported in one sentence; a partial block leaves that method to the trait's
default with `#[dispatch(default)]`, as a hand does, and the rest is
forwarded.
