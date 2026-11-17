# Limits

A [limit](glossary.md#limit) is something `whichever` cannot forward: a
kind of trait method, variant or option that the attribute refuses. This
page lists every one of them.

When you hit a limit, you get a one-sentence error on the item at fault:
the variant, the trait method, the field or the option. Where there is
something to do instead, the error says what. The rest of the enum still
compiles, so you see the error for the thing to fix, and no wall of
follow-on errors.

Every limit has a compile-fail case in
[`tests/ui`](../tests/ui), with the exact error text.

Have an error message and want the fix? Start at
[troubleshooting.md](troubleshooting.md). It goes from the message to the
cause, and covers the errors that rustc writes, too.

Each limit below has three parts: **the limit, in bold**; why it exists;
and what to do instead.

On this page:

- [The enum and its variants](#the-enum-and-its-variants)
- [Receivers](#receivers)
- [`Self` in arguments and return types](#self-in-arguments-and-return-types)
- [Methods that return `impl Trait`](#methods-that-return-impl-trait)
- [Routes, pointers and `deref` variants](#routes-pointers-and-deref-variants)
- [Trait objects and `Self: Sized`](#trait-objects-and-self-sized)
- [Associated types and consts](#associated-types-and-consts)
- [Forward to every variant, with arms of your own](#forward-to-every-variant-with-arms-of-your-own)
- [Partial impl blocks](#partial-impl-blocks)
- [Inherent methods](#inherent-methods)
- [Structs](#structs)
- [Conversions](#conversions)
- [`#[cfg]`](#cfg)
- [Names, scope and imports](#names-scope-and-imports)
- [Lints](#lints)
- [The carrier macro](#the-carrier-macro)
- [When and where you see the error](#when-and-where-you-see-the-error)
- [Errors that are not limits](#errors-that-are-not-limits)

## The enum and its variants

- **A dispatch enum needs at least one variant.** An enum with no
  variants has nothing to forward to. Add the variants, or leave the
  attribute off.
- **Every variant a trait is forwarded to holds exactly one unnamed
  field, `Name(Payload)`, or marks the field to forward to with
  `#[dispatch]`.** The forwarding arm needs one value to call the method
  on. Write the variant as `Name(Payload)`, or mark one field with
  `#[dispatch]`. You get the error under the variant's own `#[cfg]`, so a
  variant that is compiled out, such as a test-only mock, is no error.
- **Every [target](glossary.md#target) implements the trait.** The arm
  calls the trait method on it, as a hand-written arm would. Implement
  the trait for the payload, or reach a target that implements it with a
  [route](routes.md).
- **An enum that only forwards [inherent methods](inherent-methods.md)
  may hold variants of any other form.** Those variants have no payload
  to forward to, so they take the method's body.
- **The marked field of a variant takes `deref` and `via = route`
  only.** The other variant options belong to the variant itself:
  `from` and `try_from` go on the variant, not on its field.
- **A variant's marked field cannot list a trait**, as a struct's field
  may. The variant is already the target of every trait its enum
  forwards, so the trait would say nothing new, and you get an error. A
  trait that needs its own route through the variant gets it on the
  enum: `via(Variant = route)`.

## Receivers

- **A receiver is `self`, `&self`, `&mut self`, `self: Box<Self>`,
  `self: Pin<&Self>` or `self: Pin<&mut Self>`.** `self: Box<Self>` is
  forwarded only to `deref` variants that hold a `Box`. The two `Pin`
  receivers need every payload to be `Unpin`.
- **`Rc<Self>`, `Arc<Self>` and `Pin<Box<Self>>` receivers are not
  supported.** Forwarding them would need a refcount or an allocation,
  which a hand-written `match` does not do.
- **A method without a receiver, such as `fn new() -> Self`, needs a
  default body or an implementation of your own.** It has no value to
  match on, so there is no variant to forward to. Give the trait method
  a default body, which the enum then uses, or write the method in a
  [partial impl block](partial-impl-blocks.md).

## `Self` in arguments and return types

- **A trait method's `Self` argument is `Self`, `&Self` or `&mut Self`,
  and nothing else.** A `Self` nested in another type, such as
  `other: Option<&Self>`, is an error. The argument is matched beside
  `self`, on the same variant, and only a plain `Self`, `&Self` or
  `&mut Self` can be matched that way. Write such a method yourself in a
  [partial impl block](partial-impl-blocks.md).
- **A method whose return type wraps `Self` in anything but `Option`,
  `Result` or a tuple cannot be forwarded.** An example is a combinator,
  `fn buffered(self) -> Buffered<Self>`. The payload returns
  `Buffered<Payload>`, which is not the enum's `Buffered<Self>`. If the
  method is provided, leave it to the trait's default with
  `#[dispatch(default)]` in a [partial impl block](partial-impl-blocks.md).
  The default then wraps the enum. Otherwise the error names the fix
  that fits: implement the method by hand, or, for an inherent method,
  give it a body.
- **`Option` and `Result` count only by those names, and a tuple only
  outside them.** The return type that can be forwarded is `Self`,
  `Option<Self>`, `Result<Self, E>`, or a tuple of those, such as
  `(Self, u32)` or `(Option<Self>, u32)`. So:
  - `Option` and `Result` are recognised by those names, under any path,
    so `io::Result<Self>` works. An alias with another name, such as
    `Fallible<Self>` for `Result<Self, u8>`, is an error: write
    `Result<Self, u8>` in the signature.
  - A tuple inside an `Option` or a `Result`, such as
    `Option<(Self, u32)>`, is an error. Rewrapping it would need a
    closure, which the attribute does not write. Implement the method by
    hand.
- **Under `with = Some` or `with = Ok` on an inherent method, the return
  type cannot wrap `Self` twice**, as in `Option<Option<Self>>`. The
  payload's result is rebuilt into the variant and then lifted once, so
  there is no second layer to fill. Drop the method's `#[dispatch(..)]`
  and let its body return a value for every variant.
- **A variant whose one field is named and marked,
  `Named { #[dispatch] shape: Square }`, cannot forward a method that
  returns `Option<Self>` or `Result<Self, E>`.** Inside an `Option` or a
  `Result`, the value is rebuilt with a path, and no path builds a
  variant with a named field. Write it as a tuple variant,
  `Named(Square)`. A method returning plain `Self` is fine: the braces
  rebuild the variant. A struct with one named field has the same limit,
  under [Structs](#structs).

On an enum with more than one variant, a `Self` argument also needs a
`mismatch`, the value to return when `self` and the argument hold
different variants. That is an option, not
a limit: see [options.md](options.md).

## Methods that return `impl Trait`

- **An enum cannot forward a method that returns `impl Trait`**, at any
  depth of the return type. Each variant returns a different hidden
  type, and a `match` needs one. The exception is
  `impl Future<Output = T>`, which is awaited like an `async fn`.
  Otherwise, return a concrete type or a boxed trait object.
- **A struct forwards any `impl Trait`.** It forwards to one field, so
  there is only one hidden type.

## Routes, pointers and `deref` variants

A [route](routes.md) is the steps from the payload to the target: `*`
(deref), `.field` or `.method()`. A `deref` variant is the route `*`.

- **A method step in a route takes no arguments.** `.second()` is a
  step; `.get(1)` is an error. Give the payload a method without
  arguments that returns the target, and route through that.
- **A variant reached through a route, `deref` included, cannot take part
  in a method that returns `Self`.** There is no way to put the result
  back behind the pointer, or back through the route. Drop the route on
  that variant (`deref` or `via`), or implement the method by hand.
- **Neither can a variant with other fields beside the marked one.** The
  macro has no values for the other fields, so it cannot build the
  variant around the result. Write the method in a
  [partial impl block](partial-impl-blocks.md). Where the method is
  under `*` with arms you write, write that variant's arm in the
  method's body instead.
- **A route that ends in a method call cannot serve a method that borrows
  mutably**: through `&mut self`, `self: Pin<&mut Self>` or a `&mut Self`
  argument. The error says which of the three it is. The method call
  returns a value that cannot be borrowed mutably. Route to a field
  instead, or implement the method by hand.
- **A `deref` variant over an `Rc`, an `Arc` or a `&T` cannot serve a
  method that borrows mutably.** Those pointers lend what they point at
  shared only. Hold a `Box` or a `&mut`.
- **An inherent method cannot return a place as `&mut` through such a
  variant** (an `Rc`, an `Arc` or a `&T`), for the same reason. Hold a
  `Box` or a `&mut`.
- **A trait method that takes `self` or a `Self` argument by value cannot
  be forwarded to a `deref` variant that holds anything but a `Box`.**
  The method takes its target by value, as its signature says, and a
  value cannot be moved out of an `Rc`, an `Arc` or a `&T`. You get the
  error on the payload, naming the variant whose value would move. Hold a
  `Box`, give the method a body, or implement the trait method by hand.
- **An inherent method's `value` or `field` under `self` or
  `self: Box<Self>` cannot move out of such a variant**, unless the
  return type borrows the place. For the same reason, you get the error
  on the payload, naming the variant. Hold a `Box`, or give the method a
  body.
- **A method that takes `self: Box<Self>` cannot be forwarded to such a
  variant.** There is no box to hand over. Hold a `Box`, give the method
  a body, or implement the trait method by hand.
- **Not a limit: an inherent method forwarded as a call (no `field` or
  `value`) under `self`.** Its arm is what you would write by hand,
  `(*inner).name(..)`, and the payload's method decides what moves. A
  payload method taking `&self` borrows what the variant points at. One
  taking `self` gets rustc's E0507. A reference it returns out of an
  `Rc` that the arm drops gets rustc's E0515.

For the moves above, the error also suggests the fix that fits the
method:

- for a method that lists its variants and has a body for the rest, the
  error suggests moving the variant into that body;
- for a method under `*` whose body has arms you write, the error
  suggests writing the variant's arm there.

## Trait objects and `Self: Sized`

- **A method that requires `Self: Sized` cannot be forwarded to a `deref`
  variant that holds a trait object.** A trait object is not `Sized`, so
  the method cannot be called on it. Leave the method to the trait's
  default with `#[dispatch(default)]` in a
  [partial impl block](partial-impl-blocks.md); the rest is forwarded.
- **Where the pointer implements the trait itself, hold it without
  `deref`.** `Box<dyn Iterator>` and `&mut dyn Iterator` implement
  `Iterator`, so a plain variant holding one gets every method forwarded
  to it, with no partial impl block. `deref` stays the way for a trait
  the pointer does not implement, such as your crate's own
  `Box<dyn Shape>`.
- Every method of `whichever::foreign::Iterator` except `next`,
  `size_hint` and `nth` has the `Self: Sized` bound.

## Associated types and consts

- **An associated type or const needs a variant that can supply it in
  every build.** A variant under `#[cfg]` may be compiled out, and the
  type of a target reached through a field or a method cannot be named.
  So when every variant is one or the other, no variant can supply the
  item, and you get an error on the enum for each such item. Set it on
  the enum with an [override](glossary.md#override), `Tag = ..`. The
  same goes for a struct whose field is reached through a field or a
  method: set the associated type on the struct.
- **An override, `Name = Type`, converts what a method returns, not an
  argument that uses the overridden type.** The argument is handed to
  the payload as it is. So the method compiles only where every
  payload's own type is the override's; elsewhere, you get a type error
  from rustc in that arm. The methods of `foreign::Iterator` that take a
  closure over `Self::Item` are such methods under `Item = Wide`.
  Restate the trait with only the methods to forward, or leave the
  method to the trait's default with `#[dispatch(default)]` in a partial
  impl block.
- **A generic associated type comes from the enum or the struct, in a
  partial impl block.** It cannot come from the first variant or from
  the struct's field, and no option on the attribute gives it. Write
  `type Item<'a> = ..;` in a `#[whichever::dispatch] impl` block.
- **An associated const is checked against every variant only in some
  cases.** An associated const is taken from the first variant without
  a check when:
  - its type is anything but an integer, a float, `bool`, `char` or
    `&str`;
  - or the enum has a type or const parameter.

  And a variant reached through a field or a method is left out of the
  check: its const is not compared with the first variant's.
- **A projection through a trait parameter with several bounds cannot
  be qualified at the enum.** With `F: Format + Clone`, `F::Value` has
  no single bound to say which trait `Value` belongs to. Write
  `<F as Format>::Value` in the trait.
- **A partial impl block cannot declare an associated type or const
  under `#[cfg]`.** The forwarded methods convert into the type in every
  build, so it must exist in every build. Put the whole block under the
  `cfg`, with a second block for the other builds.

## Forward to every variant, with arms of your own

Under `#[dispatch(*)]` a method is forwarded to every variant. Its body
may be one `match self` with arms for some variants, written by path,
`Self::Name(..) => ..`. Those are the arms the enum handles itself; every
other variant is forwarded. See
[partial-impl-blocks.md](partial-impl-blocks.md#forward-to-every-variant-but-a-few)
and [inherent-methods.md](inherent-methods.md#forward-to-every-variant-but-a-few).

- **A method under `*` cannot be forwarded to a variant without a
  payload.** You get the error on the method, naming the variant. Write
  that variant's arm in the method's body, as one `match self` with
  `Self::Empty => ..`. Where the method cannot take such an arm (the
  next two bullets), list the variants that have a payload in
  `#[dispatch(..)]` and give the method a body for the rest.
- **A trait method with a `Self` argument cannot have arms you write
  yourself under `*`.** The `Self` argument is matched beside `self`,
  and an arm you write would be for `self` alone. List the variants in
  `#[dispatch(..)]` and give the method a body for the rest. An inherent
  method passes a `Self` argument on as it is, so this does not apply to
  it.
- **A method with a reference receiver cannot have arms you write
  yourself under `*` when every variant of the enum is under
  `#[cfg]`.** That covers `&self` and `&mut self`, plain or pinned. Such
  a method is matched as `match *self` with `ref` bindings, and the arms
  are written for `match self`. List the variants in `#[dispatch(..)]`
  and give the method a body for the rest.
- **A body under `*` must be one `match self`.** Any other body would
  never run, because every variant is forwarded, so you get an error.
- **Each arm you write names its variants by path, `Self::Name(..)`.**
  An arm that names none, `_` or a binding, is an arm for every other
  variant, and that is what a list of variants with a body already
  says. So you get an error. List the forwarded variants in
  `#[dispatch(..)]` instead, and make that arm's result the method's
  body.

## Partial impl blocks

A [partial impl block](partial-impl-blocks.md) is a trait impl with
`#[dispatch]` on it: you write some items, and the rest are forwarded.

- **An option of a trait's impl goes after the trait, or on the block's
  attribute.** An override (`Name = ..`), `mismatch(..)` or `via(..)` on
  an enum's or a struct's attribute that lists no trait would be read by
  no impl, so you get an error on the option. Write it after the trait,
  `#[whichever::dispatch(Trait, mismatch(..))]`, or on the attribute of
  the partial impl block that implements the trait. Where the enum or the
  struct lists exactly one trait on its own attributes, the option is
  read beside that trait as well, so the error stands alone. A name close
  to one of the enum's or the struct's own options, `form = false`, is
  reported as that option misspelled, and is not read.

- **A macro invocation in a partial impl block is not read.** The items
  it writes could not be told apart from the ones to forward, so you get
  an error. Write the items out in the block.
- **A macro invocation under `#[cfg]` gives an error only where the
  `cfg` is on.** Where it is off, the invocation is not in the block.
  The block is read without it and forwards as usual, as it does for any
  item of the block that cannot be read.

The `#[cfg]` rules for the other items a block cannot read are under
[When and where you see the error](#when-and-where-you-see-the-error).

## Inherent methods

- **An inherent method without a receiver cannot be forwarded.** It has
  no value to match on, so there is no variant to forward to. Leave the
  `#[dispatch(..)]` off and give it a body: methods without one are left
  alone.
- **An inherent method cannot list a variant without a payload.** There
  is nothing in it to forward to. Leave the variant out of the list; it
  runs the method's body.
- **A forwarded parameter is a plain name.** A pattern, such as
  `(x, y): (u32, u32)`, is an error, because the arm passes each
  parameter on by its name. Take the value as one named parameter,
  `point: (u32, u32)`.
- **`with` takes the path of a function, not a closure.** `with` calls
  that function on each arm's result. Write a helper function instead,
  generic over the result when the payloads' results differ.
- **Another crate's enum or struct gets no inherent block.** The language
  allows an inherent impl only in the type's own crate, so a
  [restated](restatements.md#enums-from-other-crates) enum or
  [struct](restatements.md#structs-from-other-crates) cannot have one.
  Forward through a trait of your crate, or write free functions.

The other limits of [inherent methods](inherent-methods.md) sit with the
topic they touch:

- the `with = Some` lift over a doubly wrapped `Self`, under
  [`Self` in arguments and return types](#self-in-arguments-and-return-types);
- `value`, `field` and `&mut` places through a shared pointer, under
  [Routes, pointers and `deref` variants](#routes-pointers-and-deref-variants);
- an inherent method of a struct, under [Structs](#structs);
- the `#[cfg]` rules for a method of an inherent block, under
  [When and where you see the error](#when-and-where-you-see-the-error).

## Structs

A [dispatch struct](glossary.md#dispatch-struct) forwards a trait to one
of its fields. See [structs.md](structs.md).

- **A dispatch struct needs a field.** A unit struct has nothing to
  forward to. Give it the field that implements the trait.
- **A dispatch struct with several fields marks the one to forward to.**
  Mark it with `#[dispatch]`, or list the trait on it,
  `#[dispatch(Trait)]`. Otherwise the attribute cannot tell which field
  is meant, and you get an error.
- **A dispatch struct with other fields beside the marked one cannot
  forward a method that returns `Self`.** The macro has no values for
  the other fields, so it cannot build the struct around the result.
  Write the method in a [partial impl block](partial-impl-blocks.md).
- **A trait is forwarded to one field.** A method of a partial impl
  block that names another field, or a route on the attribute keyed by
  another field, `via(holder = ..)`, is an error. To forward the trait
  to that field, list the trait on it, `#[dispatch(Trait)]`.
- **An inherent method of a dispatch struct forwards to one field,
  `#[dispatch(square)]`.** Every value holds every field, so a second
  field in the list would be an arm no value reaches, and you get an
  error. Write a method that needs two fields by hand.
- **On a struct whose fields all list their traits, `#[dispatch(*)]` on
  an inherent method names no field.** `*` means the field marked with
  a bare `#[dispatch]`, and there is none. Mark the field to forward to
  with a bare `#[dispatch]`, or name it on the method,
  `#[dispatch(field)]`.
- **A dispatch struct whose one field is named,
  `struct Named { shape: Square }`, cannot forward a method that returns
  `Option<Self>` or `Result<Self, E>`.** Inside an `Option` or a
  `Result`, the value is rebuilt with a path, and no path builds such a
  struct. Make it a tuple struct, `struct Named(Square);`. A method
  returning plain `Self` is fine either way: the braces rebuild it.
- **A field lists a trait exactly as the impl's home spells it.** The
  home is the struct's attribute or a partial impl block's header. A
  macro cannot resolve names, so it compares spellings. A home whose
  trait no field lists as it is spelled there, next to a field that
  lists the same name another way, is an error. Another way means more
  or fewer path segments, `Catalog` and `traits::Catalog`, or with or
  without its generic arguments, `Encoder` and `Encoder<F>`. Spell it as
  the home does. The impl is still written for that field, so you get no
  follow-on errors.
- **Two different traits with the same name look like one.** A `Named`
  and a `wire::Named` are taken for one name spelled two ways. Import one
  of them under another name to tell them apart.
- **A struct takes no conversion options.** `from`, `try_from` and
  `into` are options of an enum; a struct gets no conversions, so they
  are an error on it.
- No error: a field's mark for a trait that no home implements. The
  struct does not see its impl blocks, so it cannot tell. The trait's
  first use says that the impl is missing.

## Conversions

The `From`, `TryFrom` and `into` impls belong to the enum and are written
once, whatever attributes it has. See [options.md](options.md).

- **`from = true` on one of the enum's attributes and `from = false` on
  another contradict each other.** You get the error on the later one.
  Give `from` one value, or write it once.
- **An `into` target can be written only once**, whether on two of the
  enum's attributes or twice on one. It would be the same
  `From<Enum> for Target` written twice. You get the error on the second.
  Write each target once.
- **A payload that is a type parameter of the enum gets no `From`.**
  `From<T>` would overlap the `From` impls of the other variants. Add
  `from = false` on the enum, or `#[dispatch(from = false)]` on that
  variant.
- **Nor does it get `TryFrom` under `try_from`.** `TryFrom<Enum> for T`
  would implement a foreign trait for a type parameter, which the orphan
  rules forbid. Add `#[dispatch(try_from = false)]` on that variant.
- **A payload that is an associated type of a type parameter,
  `Out(S::Out)` or `Out(<S as Source>::Out)`, gets no `From`, even when
  it is the only variant.** `S::Out` may be any type, the enum itself
  included, so `From<S::Out>` would overlap `core`'s `From<T> for T`. A
  hand cannot write that impl either. Add `from = false` on the enum, or
  `#[dispatch(from = false)]` on that variant.
- **Nor does it get `TryFrom` under `try_from`.** For the same reason,
  `TryFrom<Enum> for S::Out` would overlap the `TryFrom` that `core`
  implements for every type. Add `#[dispatch(try_from = false)]` on that
  variant.
- Not a limit: a projection from a type that is not a type parameter,
  such as `<Fixed as Source>::Out`. rustc can name that type, so the
  payload gets its `From` and `TryFrom` like any other.
- **`into = Target` needs every variant to hold a payload.** It converts
  every variant's payload, so a variant without one, or with other
  fields beside its payload, is an error. Write the `From<Enum> for
  Target` impl by hand instead.
- **A restated enum takes no conversion options**, not even
  `from = false`. `From`, `TryFrom` and `into` would implement a foreign
  trait for another crate's type, which the orphan rules forbid, so it
  gets no conversions at all. Leave the options off. A restated struct
  gets the error any struct gets for them.
- **`remote` goes on the enum's or the struct's own attribute.** It
  restates the type, and a partial impl block's attribute takes the
  trait's options only, so `remote` there is an error.
- **A restated struct's pattern ends in `..`**, so a field the
  restatement leaves out is not caught. `-> Self` built around a struct
  that has such fields fails against the real struct.

## `#[cfg]`

- **A `mismatch` on the enum lists a method the trait has in every
  build.** A method the trait declares under a `#[cfg]` that is off is
  not in that build's trait, so the option names nothing and you get an
  error. Put the mismatch on the trait method,
  `#[dispatch(mismatch = ..)]`, which travels under the method's `cfg`.
  For another crate's trait, write the enum's attribute twice, under
  `cfg_attr` with the trait's predicate.
- **A trait's items carry at most four distinct `#[cfg]` predicates.**
  The carrier macro is written once per combination of them, so you get
  the error on the fifth. Combine the predicates, or move the items under
  the fifth to a trait of their own.

- **A mark cannot go under `cfg_attr`.** That covers
  `#[cfg_attr(feature = "x", dispatch)]` on a struct's field, a variant, a
  variant's field or a method of a block. An attribute macro is handed its
  item before a `cfg_attr` inside the item is decided, and a macro cannot
  decide a predicate itself, so such a mark would be read in no build. You
  get an error on the mark, and the item is read as if the mark were
  plain, so nothing else follows. Instead:
  - a struct marks its field in every build, with a bare
    `#[whichever::dispatch]` beside the trait's attribute under
    `cfg_attr` (see [structs.md](structs.md#a-trait-in-some-builds-only));
  - a variant or a method whose mark differs between builds is written
    twice, under `#[cfg(..)]` with the mark and under `#[cfg(not(..))]`
    without it.

An associated type or const under `#[cfg]` in a partial impl block is
under [Associated types and consts](#associated-types-and-consts).

## Names, scope and imports

- **The types in the trait's signatures must be in scope where the enum
  is**, as for a hand-written impl. They are written into the enum's
  module. Paths that start with `crate::` are rewritten to the trait's
  crate; `super::` and `self::` paths are not.
- **They must also be nameable from the enum's module.** A private alias
  or type in a signature limits the trait to its own crate.
- **The attribute does not import them for you.** A glob import written
  around the impl would shadow the enum module's own names. A glob import
  of the trait's crate in the enum's module, `use trait_crate::*;`,
  brings them in, and the module's own names still win over it. To keep
  the glob out of the enum's file, put it in a module that writes one
  empty [partial impl block](partial-impl-blocks.md) per trait,
  `#[whichever::dispatch] impl Client for ClientWrapper {}`, with the
  payload types imported there by name.
- **A private glob import must not bring the trait's own name back into
  the trait's module.** An example is `use crate::prelude::*;` at a crate
  root whose prelude re-exports a trait declared there. It makes the
  carrier macro's re-export ambiguous to rustc
  (`ambiguous_import_visibilities`, an error under
  `#![deny(future_incompatible)]`). Declare the trait in a module of its
  own and re-export it, or import the prelude's items by name there.

## Lints

- **A lint attribute on the enum does not reach the impls written next to
  it**, just as it would not reach a hand-written impl. Put `#[allow]` on
  the trait method, on a partial impl block, or on the module.

## The carrier macro

The [carrier macro](glossary.md#carrier-macro) is the hidden macro
`#[dispatchable]` leaves next to the trait, holding its signatures.

- **A macro invocation inside a `#[dispatchable]` trait is not
  supported.** The carrier macro holds the trait's items as they are
  written, and the items a macro would write are not among them. Write
  the items out in the trait.
- **One `macro_rules!` may emit a `#[dispatchable]` trait in several
  modules only when at least one token of the trait comes from the
  invocation**, such as its name or a method's name. A trait written out
  in full in the macro's body gets the same carrier macro name at every
  invocation, and rustc says the name is defined multiple times. Give
  each invocation its own name with `#[dispatchable(name = ..)]`, for
  example `#[dispatchable(name = tokio)]`.

## When and where you see the error

Everything the attribute cannot forward gets a one-sentence error on the
offending trait item or variant; [`tests/ui`](../tests/ui) has all of
them. These rules decide where and when you see it:

- **An error about one variant follows that variant's `#[cfg]`.** That
  covers both the variant's form and the way a method reaches it. A
  variant that is compiled out is no error, as a hand-written arm for it
  would not exist either.
- **Each variant a method cannot reach gets an error of its own.** So you
  see every variant to fix at once, each by name.
- **An error about a method of an inherent `#[dispatch]` block follows
  that method's own `#[cfg]`.** That covers a method without a receiver,
  a mark that cannot be read, and a method without a body and without a
  mark. A method that is compiled out is no error, as a hand-written
  method under that `cfg` would be absent.
- **Tokens in an inherent block that are not a method give an error in
  every build**, since there is no method whose `cfg` they could follow.
- **An item of a partial impl block that cannot be read also follows its
  own `#[cfg]`.** That covers a method without a body and without a
  mark; a mark that does not parse, carries an inherent method's option,
  or stands next to a second mark; `#[dispatch(default)]` on a method
  with a body; and a macro invocation. Where the `cfg` is off, the block
  is read without the item and forwards as usual.
- **A rejected enum is still emitted**, so the derives, `use`s and impl
  blocks around it still compile.
- **A trait method that cannot be forwarded is still written, with the
  error as its body.** So the impl is complete and the calls to it
  resolve.
- **An associated type or const that no variant can supply is still
  written, holding the error.** Again, the impl is complete. Together
  with the two bullets above, this means you get no follow-on errors.

## Errors that are not limits

Most of these errors come from rustc, about the code the attribute
wrote. They read as they would on a hand-written impl.
[troubleshooting.md](troubleshooting.md) has a full entry for each:

- **"cannot find macro `Shape` in this scope"** at the enum: the trait is
  not marked `#[dispatchable]`, or the trait (which carries the macro) is
  not imported where the enum is.
- **A `std::`, `core::` or `alloc::` path in `#[dispatch]`** gets an
  error of its own, naming the `whichever::foreign` restatement or the
  `remote` restatement to write instead.
- **"cannot find `Trait` in `other`"** on `#[dispatch(other::Trait)]`
  (older Rust, such as 1.88, words it "could not find `Trait` in
  `other`") means the same for a trait of another crate: restate it with
  `remote`.
- **The same words on a partial impl block over another crate's enum**,
  `#[whichever::dispatch] impl Local for other::Enum {}`, mean one of two
  things:
  - the enum has no `#[whichever::dispatch]`, and only its own crate can
    add it;
  - or the enum is another crate's, and you restate it with `remote`
    ([Enums from other crates](restatements.md#enums-from-other-crates)).
- **A carrier macro "defined multiple times"**: one `macro_rules!` emits
  the same `pub` trait, written out in full, in two places, so both get
  the same carrier macro name. `#[dispatchable(name = tokio)]` names the
  macro explicitly and tells the two apart. See
  [The carrier macro](#the-carrier-macro).
