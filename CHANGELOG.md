# Changelog

Notable changes, written so a person can read them. Before 1.0, breaking
changes are expected and listed without apology.

## Unreleased

## 0.5.1 - 2026-11-02

A patch for the nine bugs the ambassador and enum_delegate surveys filed
against 0.5.0, and for the gap items whose design was settled. Nothing that
compiled before is read differently, except an inherent method of a struct
whose `Self` argument now means the field. The generated code of the
benchmarked shapes is unchanged, so the numbers stand.

- A struct forwards a return-position `impl Trait` as it is: its `match`
  has one arm, so there is one opaque type. A future whose output needs
  nothing is forwarded without an `async` block; one whose output is
  rewrapped or converted is awaited in one, as on an enum. An enum still
  rejects every opaque return but a future, now at any depth of the return
  type; `Result<impl Stream, E>` reached rustc before. The decision record
  is superseded accordingly.
- A rejected attribute no longer takes the others with it. The unit of
  rejection at an enum or struct is the attribute: the ones that parse are
  expanded, a conversion that cannot be written leaves the other
  conversions standing, and a partial impl block of such an enum is
  forwarded in full. Before, one bad option in a stack of three gave one
  real error and a follow-on at every use, and a partial block over an enum
  with a `From` overlap gave E0046 on top.
- A partial impl block of a trait the enum's own attribute dispatches is
  reported on the block as the second impl it would be; rustc reported
  conflicting implementations on the trait's attribute.
- An error rustc finds in one trait's generated impl is labelled on that
  trait's attribute, not on the first of the stack.
- An inherent method of a struct unwraps its `Self`, `&Self` and `&mut Self`
  arguments to the field, as trait dispatch on a struct does; the argument
  was handed to the field's method as the struct, and rustc reported a type
  mismatch. An enum's inherent method hands a `Self` argument over as it
  is, as before, since there `Self` is the enum.
- An `#[allow]` written on a trait method or associated item is carried
  onto the generated item, which repeats the signature the lint was allowed
  on; `#[cfg]` alone travelled before. One another macro added stays where
  it was, since rustc exempts it from the lints about lint names only
  there.
- A partial impl block's header may name the enum's parameters as it
  likes, `impl<T, Ty> Get<T> for ImmOrNode<Ty>` over `enum ImmOrNode<T>`;
  the arms followed the enum's names and rustc reported an unsatisfied
  bound at the variant.
- A generic associated type declared in a partial impl block is accepted
  and every payload's own converts into it, as for a plain override. The
  behaviour had landed with the 0.5.0 refactor unannounced; it now has its
  tests, its smoke case across crates, and its sentence: an undeclared one
  is pointed at the block to write, and `Name<..> = ..` on the attribute at
  the same, where it read as an unknown option.
- An override or a `mismatch` naming nothing in the trait says what was
  probably meant: the associated item or method within an edit or two, the
  enum option `from` or `try_from` for a typo of it, what chooses the field
  or the variants for `target`, `to`, `field`, `delegate` and `via`, and
  otherwise everything there is to name.
- A variant that is not `Name(Payload)` is reported with what it holds
  instead, "holds no payload", "holds 2 fields, not one payload" or "has
  named fields", and with inherent dispatch as the way around it, under `*`
  as well. An enum whose every variant is rejected no longer gets "needs at
  least one variant" on top.
- `#[dispatchable]` reports `#[async_trait]` below it, which rewrites the
  signatures after the carrier has read them, and `#[typetag::serde]` above
  it, whose per-impl methods a forwarder would answer with the payload's
  tag, silently; each with the order to use.
- Reported as one sentence now: a return type mentioning `Self` where it
  cannot be rewrapped, naming the struct or enum and the way out of the
  method's own path, where it said "the enum" on a struct; a projection
  `F::Value` through a trait parameter with several bounds, which rustc
  reported as an unknown type; a second `#[dispatch]` on one inherent
  method; a closure after `with`; the number and names of a generic trait's
  parameters when none is given; and the partial block a receiverless
  method goes in, spelled with the trait's arguments and the enum's
  generics.
- The README names the layout a private prelude glob makes ambiguous for
  the carrier's re-export, with the ways around it; says that the `TryFrom`
  error being the enum asks for `Debug` under `.unwrap()`; that a payload
  type held by two variants gets no `From`; that a restatement's own imports
  count as unused and its paths go in full; that an inherent arm uses
  method-call syntax and `with` may name a generic function; that a generic
  trait may be dispatched at the enum's own parameters; and what "could not
  find `Trait` in `other`" means. The struct section is "Forwarding from a
  struct field" and shows a field that is not an enum.
- The design is written down under `docs/`: the vision with its five rules,
  what holds each of them, and what the crate will not be; a decision
  record per choice made among real alternatives, with the condition that
  would reopen it; and an index of what to read when. `ACKNOWLEDGEMENTS.md`
  credits the crates that came before, one lesson each; `RELEASING.md` is
  the release checklist; `CLAUDE.md` holds the rules of the project for a
  coding agent. The macro that `#[dispatchable]` leaves next to a trait is
  the carrier macro throughout; the README and the crate docs had called it
  the hidden macro.

## 0.5.0 - 2026-11-02

Both crates move to 0.5.0 together; `whichever` pins `whichever-macros` to
the exact version from here on.

- A method returning `Result<Self, Self::Error>` with `Error` fixed on the
  enum rewraps the value and converts the error; before, the error was left
  as the payload's and rustc reported a type mismatch at the trait's
  attribute. `Option<Self::X>` and `Result<Self::X, E>` with `X` fixed
  convert too, and a `Self` in a `Result`'s error position is reported as a
  limit.
- The forwarding impl keeps the trait's parameter names, so `fn write(&mut
  self, buf: &[u8])` reads as written instead of `__argument0`; a parameter
  whose pattern is not a plain identifier still gets a generated name.
- Every trait item that cannot be forwarded is reported, not only the
  first.
- A const fixed on the enum may be written as a tuple of paths, `RANGE =
  (LOW, HIGH)`; it was read as a type and rejected.
- A trait path in the `#[dispatch(..)]` of a partial impl block is
  reported; the trait is the one the block implements.
- The macro crate is split into modules, described in
  `macros/ARCHITECTURE.md`. The carrier macros' names come from a fixed
  hash now, so the hidden name a `pub` trait exports changes once with this
  release and then stays the same across compiler releases.
- An `unsafe trait` is implemented by an `unsafe impl`; it got a plain
  `impl`, which rustc rejected.
- A trait method's own `#[inline(..)]` is carried onto the generated method;
  it was dropped, and the method got no `#[inline]` at all.
- A generic method whose type or const parameter no argument fixes is
  called with the parameters spelled out; before, rustc asked for type
  annotations inside the generated code.
- The associated types and consts come from the first variant that is not
  under `#[cfg]`, and an enum whose every variant is gets one sentence
  asking for an override. An associated item under `#[cfg]` in the trait
  stays under it in the impl.
- A `mismatch` expression written on the trait method may use a `Self` or
  `&mut Self` argument; the fallback arm binds it again under its name.
- An error rustc finds in the generated code points at the enum or at the
  trait method, not at the trait's `#[dispatchable]` attribute, which may
  be in another crate.
- Reported as one sentence now: an override or `mismatch` naming nothing in
  the trait, an option or a variant given twice, an associated type bound
  inside the trait's path, `try_from = true` on a type-parameter payload,
  `mismatch` on a method without a `Self` argument, two `#[dispatch]`
  attributes on one trait method, a bare `#[dispatch]` on an inherent
  method, and a trait path in a partial block's attribute. The `Box<Self>`
  message spells the payload as written and says to add `deref` when the
  payload is a `Box` already.
- A rejected `#[dispatch]` on an inherent impl block keeps the block, so
  its methods resolve and the one error stands alone.
- `whichever` pins `whichever-macros` to the exact version: the hidden
  `__private` module and the generated code are one interface, and the two
  crates are released together.

## 0.4.0 - 2026-11-02

- `#[dispatch(Trait)]` on a struct forwards the trait to its one field, or
  to the field marked `#[dispatch]`, provided methods included, so a
  payload's override reaches a public wrapper over a private dispatch enum.
  `#[dispatch(deref)]` on the field reaches through a pointer, a method
  returning `Self` is rebuilt around a single-field struct, and partial and
  inherent impl blocks work on a struct as on an enum.
- `self: Pin<&Self>` and `self: Pin<&mut Self>` receivers dispatch when
  every payload is `Unpin`, and `self: Box<Self>` dispatches to `deref`
  variants holding a `Box`, handing the payload's own box over. `Rc<Self>`,
  `Arc<Self>` and other receivers stay rejected, with a message naming the
  supported ones.
- A variant that is not `Name(Payload)` is reported under its own `#[cfg]`,
  so a variant compiled out, such as a test-only mock, is no error. An enum
  that only forwards inherent methods may hold such variants: a method
  naming the variants it forwards to leaves them to its body, and `*` over
  such a variant is an error on that variant.
- A payload that names the enum as `Self`, such as `Wrap<Self>`, works with
  `try_from = true` and with the associated type and const checks; the
  generated items outside the enum's own impls named `Self` where it meant
  nothing, and rustc rejected them.
- `#[dispatch(try_from = true)]` on a variant adds the `TryFrom` impl for
  that payload alone.
- A payload that is a bare type parameter of the enum, next to other
  variants' payloads, is reported as overlapping their `From` impls, with
  `from = false` named as the fix; before, rustc reported conflicting
  implementations on the attribute.

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
