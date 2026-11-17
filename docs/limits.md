# Limits

- Every variant a trait is dispatched to holds exactly one unnamed field,
  `Name(Payload)`, or marks the field to forward to with `#[dispatch]`,
  and every target implements the trait. A variant of another shape is
  reported under its own `#[cfg]`, so a variant compiled out, such as a
  test-only mock, is no error; an enum that only forwards inherent methods
  may hold such variants, which take the method's body.
- A method under `#[dispatch(*)]` over a variant without a payload is
  reported on the method, naming the variant. Write the variant's arm in
  the method's body, one `match self`, `Self::Empty => ..`, or, where the
  method can take no such arm (the two lines below), name the variants
  with a payload and give the method a body for the rest. A body under
  `*` that is anything but one `match self` would never run, and is
  reported too.
- Under `*`, an arm the enum answers itself names its variants by path,
  `Self::Name(..)`; one that names none, `_` or a binding, is reported.
  Name the forwarded variants in `#[dispatch(..)]` and make it the
  method's body instead.
- A trait method with a `Self` argument, which is matched beside `self`,
  holds no arms the enum answers itself under `*`; an inherent method
  hands a `Self` argument on as it is and is not concerned. Name the
  variants in `#[dispatch(..)]` and give the method a body for the rest.
- A method whose receiver is a reference, `&self` or `&mut self`, plain
  or pinned, over an enum whose every variant is under `#[cfg]` is matched
  as `match *self` with `ref` bindings, so under `*` it holds no arms the
  enum answers itself, which are written for `match self`. Name the
  variants in `#[dispatch(..)]` and give the method a body for the rest.
- An enum cannot dispatch a return-position `impl Trait`, at any depth of
  the return type, except `impl Future<Output = T>`, which is awaited like
  an `async fn`; a struct forwards any.
- A variant reached through a route, `deref` included, cannot take part in
  a method returning `Self`: there is no way to put the result back behind
  the pointer or around the other fields. A route ending in a method call
  cannot serve a method that borrows mutably, through `&mut self`, `self:
  Pin<&mut Self>` or a `&mut Self` argument, and the sentence names which;
  nor can a `deref` variant over an `Rc`, an `Arc` or a `&T`, which lends
  what it points at shared only, and a place an inherent method returns as
  `&mut` through one is reported too. Hold a `Box` or a `&mut`.
- A method that requires `Self: Sized` cannot be forwarded to a `deref`
  variant holding a trait object; a partial block leaves it to the trait's
  default with `#[dispatch(default)]`. Where the pointer implements the
  trait itself, as `Box<dyn Iterator>` and `&mut dyn Iterator` implement
  `Iterator`, hold it without `deref`, and every method is forwarded to
  it. `deref` stays the way for a trait the pointer does not implement,
  such as a crate's own `Box<dyn Shape>`. Every method of
  `foreign::Iterator` but `next`, `size_hint` and `nth` has the bound.
- An override, `Name = Type`, converts what a method returns, not an
  argument that names the overridden type: the argument is handed to the
  payload as it is, so the method compiles only where every payload's own
  type is the override's, and elsewhere rustc reports the arm. The methods
  of `foreign::Iterator` that take a closure over `Self::Item` are such
  methods under `Item = Wide`. Restate the trait with only the methods to
  forward, or leave a method to the default with `#[dispatch(default)]` in
  a partial block.
- A method whose return type wraps `Self` in anything but `Option`,
  `Result` or a tuple, such as a combinator
  `fn buffered(self) -> Buffered<Self>`, cannot be forwarded: the payload's
  `Buffered<Payload>` is not the enum's. A provided one is left to the
  trait's default in a partial block with `#[dispatch(default)]`, and the
  default then wraps the enum.
- Under `with = Some` or `with = Ok` on an inherent method, the payload's
  result is rebuilt into the variant and lifted once: a declared return
  that wraps `Self` twice, `Option<Option<Self>>`, is reported. Drop the
  method's `#[dispatch(..)]` and let its body answer for every variant.
- A dispatch struct whose one field is named, `struct Named { shape: Square
  }`, cannot forward a method returning `Option<Self>` or `Result<Self, E>`:
  the value is rebuilt with a path, and no path builds such a struct. Make
  it a tuple struct, `struct Named(Square);`; `-> Self` is rebuilt by the
  braces either way. The same holds for a variant whose one field is named
  and marked, `Named { #[dispatch] shape: Square }`: write it as a tuple
  variant, `Named(Square)`.
- The marked field of a variant takes `deref` and `via = route` only:
  `from` and `try_from` go on the variant, and a trait named on the field,
  as a struct's field may name one, is reported, since the variant is the
  target of every trait its enum dispatches. A trait's own route through
  the variant goes on the enum, `via(Variant = route)`.
- A generic associated type is declared by the enum or the struct, in a
  partial impl block; it cannot come from the first variant or from the
  struct's field, and no option on the attribute gives it.
- A struct's field names a trait as its impl's home spells it, the struct's
  attribute or a partial impl block's header, since a macro cannot resolve
  names. A home whose trait no field names as it is spelled there, beside a
  field that names one with the same name in another spelling, with more
  or fewer path segments, `Catalog` and `traits::Catalog`, or with or
  without its generic arguments, `Encoder` and `Encoder<F>`, is reported:
  spell it as the home does. Two traits of one name that the sentence
  takes for one, a `Named` and a `wire::Named`, are told apart by
  importing one under another name. A field's mark for a trait no home
  implements is not reported, as the struct does not see its blocks; the
  trait's first use says that the impl is missing.
- A projection through a trait parameter with several bounds, `F::Value`
  with `F: Format + Clone`, cannot be qualified at the enum; the trait
  writes `<F as Format>::Value`.
- A `mismatch` on the enum names a method the trait has in every build: one
  the trait declares under a `#[cfg]` that does not hold is not in that
  build's trait, and the option is reported as naming nothing. Put the
  answer on the trait method, `#[dispatch(mismatch = ..)]`, which travels
  under its `cfg`; for another crate's trait, write the enum's attribute
  twice under `cfg_attr` with the trait's predicate.
- An associated type or const a partial impl block declares under `#[cfg]`
  is an error: the forwarded methods convert into the type in every build.
  Put the whole block under the `cfg`, with a second block for the other
  builds.
- A macro invocation in a partial impl block is not read, so the items it
  writes could not be told from the ones to forward. It is reported; write
  the items out in the block. One under `#[cfg]` is reported where the
  `cfg` holds; where it does not, the invocation is not in the block, which
  is read without it and forwards as usual.
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
  a `Box`: a trait method taking `self` or a `Self` argument by value, which
  takes its target as the signature says, is reported on the payload,
  naming the one that would move, and so is an inherent method's `value` or
  `field` under `self` or `self: Box<Self>` unless the return type borrows
  the place. An inherent method forwarded by name under `self` is not: its
  arm is the hand's, `(*inner).name(..)`, and the payload's method decides
  what moves, so one taking `&self` borrows what the variant points at,
  one taking `self` is rustc's E0507, and a reference it returns out of an
  `Rc` the arm drops is rustc's E0515. A method taking `self: Box<Self>` on
  such a variant is reported too, since there is no box to hand over. Hold
  a `Box`, or give the method a body, or implement a trait method by hand; a
  method that names its variants and has a body for the rest is offered
  that body, and one under `*` whose body holds the arms the enum answers
  itself is offered the variant's arm there.
- The types in the trait's signatures are written into the enum's module
  and must be in scope there, as for a hand-written impl; `crate::` paths
  are rewritten to the trait's crate, `super::` and `self::` paths are not.
  They must also be nameable from there: a private alias or type in a
  signature limits the trait to its own crate. The attribute does not
  import them itself, since a glob written around the impl would shadow
  the enum module's own names. A glob import of the trait's crate in the
  enum's module, `use trait_crate::*;`, brings them in, and the module's
  own names still win over it; to keep the glob out of the enum's file,
  a module that holds it writes one empty
  [partial impl block](partial-impl-blocks.md) per trait,
  `#[whichever::dispatch] impl Client for ClientWrapper {}`, with the
  payload types imported there by name.
- An associated const of a type other than an integer, a float, `bool`, `char` or
  `&str`, on an enum with a type or const parameter, or of a variant reached
  through a field or a method, is taken from the first variant without a
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
item or variant; see `tests/ui` for all of them. A sentence about one
variant, whether about its shape or about the way a method reaches it, is
reported under that variant's `#[cfg]`, so a variant compiled out is no
error, and each variant a method cannot reach is named in a sentence of its
own. A sentence about a method of an inherent `#[dispatch]` impl block,
one without a receiver, a mark that cannot be read or a bodiless method
without a mark, is reported under that method's own `#[cfg]`, so a method
compiled out is no error, as a hand's method under that `cfg` is simply
absent; tokens in the block that are no method are reported in every
build. A rejected enum is still
emitted, so the derives, uses and impl blocks around it compile, and a
trait method that cannot be forwarded is still written, with its sentence
as its body, and so is an associated type or const no variant can supply,
holding its sentence, so the impl is complete and the calls around it
resolve: that one error stands alone.

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
