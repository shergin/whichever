# Changelog

Notable changes, written so a person can read them. Before 1.0, breaking
changes are expected and listed without apology.

## Unreleased

What the search of 2026-11-12 over about 200 crates found. darling_core
0.24.1's error iterator is an enum over `iter::Once` and `vec::IntoIter`
that implements `Iterator` with `next` alone, so its `size_hint` is
`(0, None)` and the enum is neither double-ended nor exact-size although
both payloads are; sonic-rs 0.5.10's `IterInner` implements
`ExactSizeIterator::len` exactly while its `size_hint` reports `(0, None)`.
rodio 0.23.0's `Source` has 7 required methods and about 29 provided
combinators such as `fn buffered(self) -> Buffered<Self>`, and its
`DecoderImpl` forwards the required ones by hand over 5 `#[cfg]` variants.

- `whichever::foreign` ships `DoubleEndedIterator`, which implements
  `core::iter::DoubleEndedIterator` and forwards `next_back`, `nth_back`,
  `rfold` and `rfind`, and `ExactSizeIterator`, which implements
  `core::iter::ExactSizeIterator` and forwards `len`. `advance_back_by`,
  `try_rfold` and `is_empty` keep their defaults: the first and the last
  are unstable, and `try_rfold` needs the unstable `Try`. Each is
  dispatched beside `Iterator`, its supertrait, with one
  `#[whichever::dispatch(..)]` attribute per trait. `rfold` and `rfind`
  require `Self: Sized`, so an enum with a `deref` variant holding a
  `Box<dyn DoubleEndedIterator>` leaves them to the default in a partial
  block with `#[dispatch(default)]`, as `Iterator`'s `count`, `last` and
  `fold`. `FusedIterator` has no methods, so a restatement would declare
  the enum fused whatever its payloads are; it stays a hand-written marker
  impl.
- A provided method whose return type mentions `Self` where the payload's
  result cannot be wrapped back into the enum, such as a combinator
  `fn buffered(self) -> Buffered<Self>`, is reported with a sentence that
  first names the way a hand takes: leave it to the trait's default with
  `#[dispatch(default)] fn buffered(..);` in a
  `#[whichever::dispatch] impl Source for Any` block, written out for the
  trait and the enum. The default wraps the enum itself and calls the
  forwarded methods, so each of rodio's combinators takes one such line.
  Before, the sentence offered only spelling the return as `Self`,
  `Option<Self>`, `Result<Self, E>` or a tuple of those, or implementing
  the method by hand; a required method, a method a restatement lists
  without a body and an inherent method keep that sentence.

## 0.7.1 - 2026-11-09

A fix for inherent methods on `deref` variants, found by the survey of
Expo's Exact, whose GPU device enum forwards three methods to `Box`ed
backends by hand.

- An inherent `#[dispatch]` method on an enum with a `#[dispatch(deref)]`
  variant compiles, and the arm calls the method on what the payload points
  at: `(**inner).present(texture)`. Before, the arm was written
  `*inner.present(texture)`, which dereferences the method's result, so
  rustc reported E0614 or E0308 at the enum; `field = name` read
  `&**inner.name` and failed the same way. The method of the pointer itself,
  `Box`'s `as_ref` for one, never takes over from the payload's.
- An inherent method's route that starts with `*` reaches what the payload
  points at, as a trait's does, so `via = *.clone()` on a `Box` payload
  clones the target and no longer the `Box`. An enum whose every variant
  is `deref` and whose payloads return a reference to a `Copy` value where
  the method declares the value compiled before by dereferencing that
  result; its arms are now a type error, as they are on a plain variant.

## 0.7.0 - 2026-11-06

The review of 2026-11-06, executed. rustc's errors inside generated code
land on the enum, the method or the payload, never on the trait's
attribute; the prefix a trait's carrier macro puts in front is versioned,
so an older `whichever` reports a newer trait crate in one sentence instead
of misreading it; and deprecated items, trait object payloads, defaulted
trait parameters, same-named traits on a struct's fields, a struct's fields
in an inherent block and a boxed future's converted output are handled as a
hand writes them. The minor rises because the carried prefix changed: a
trait crate on 0.7 read by an enum crate on 0.6 is reported, and before 1.0
the trait's crate and the enum's crate use the same minor. Nothing that
compiled before is read differently, except an associated const of type
`f32` or `f64` that disagrees between the variants, which is reported now
as the integers' is. The generated code of the benchmarked shapes is
unchanged, so the numbers stand.

- Every token the generated code is written with carries a user span, the
  enum's, the method's or the payload's. Before, the patterns, the
  associated items, the agreement block and a dereferenced payload's type
  were written at the call site, which inside the implementation macro is
  the trait's `#[dispatchable]` attribute, so a rustc error about them, a
  `dyn` payload under a `Self: Sized` method, a first variant without the
  impl, a deprecated variant, was reported on the trait's attribute, in
  another crate's file when the trait is foreign. A unit test of the macro
  crate reads the engine's modules and names a token written at the call
  site, so the rule cannot rot.
- The agreement block asks the first variant for the trait's impl before
  it compares the others' associated types with its, so a first variant
  whose payload lacks the impl is "the trait bound `Memory: Storage` is not
  satisfied" on that variant; before, rustc reported a type mismatch
  resolving the enum's own projection, twice, on the trait's attribute and
  the trait method. The block is written for a struct and a one-variant
  enum too, where it holds that one check.
- A `#[deprecated]` trait method or associated item is forwarded under
  `#[allow(deprecated)]`, and the arms and conversions of a `#[deprecated]`
  variant are written under it, as a hand writes them; the deprecation
  still reaches the callers through the trait's item and the variant.
  Before, an enum dispatching a trait with a deprecated method, or holding
  a deprecated variant, could not compile under `deny(deprecated)`, since
  no attribute of the enum's author reaches the generated code.
- A method that requires `Self: Sized` forwarded to a `deref` variant
  holding a trait object, `Box<dyn Trait>` or `&dyn Trait`, is one sentence
  on the method, naming the partial block that leaves it to the trait's
  default; rustc reported that the trait object has no size, twice. The
  shape is how an enum holds an open set, and `foreign::Iterator`'s
  `count`, `last` and `fold` have the bound, so the block over a
  restatement accepts `#[dispatch(default)]` for a method the restatement
  carries no body for, leaving it to the real trait's.
- A trait parameter with a default may be left out of the trait's path on
  the enum, `#[dispatch(Enc)]` for `trait Enc<F = Json>`, as `impl Enc for
  A` leaves it out; before, the count of arguments was reported as a limit.
- A struct's field names a trait as the struct's attribute spells it,
  arguments included, so `Encoder<Json>` and `Encoder<Xml>`, or `a::Named`
  and `b::Named`, go to two fields; before, the last segment of the path
  was the trait's identity, and both were "named on two fields". One rule
  tells two traits apart wherever the macros compare them: the path as
  written, which is what a macro that cannot resolve names can say.
- The prefix a trait's carrier macro puts in front of a request is `v2`:
  0.6.0 changed what the `trait { .. }` segment holds, signatures without
  bodies, under the `v1` of the releases that held the bodies, so an enum
  crate on 0.5 reading a trait crate on 0.6 took a provided method without
  a receiver for a required one and said "give it a default body". The
  reader reads both versions; a reader too old for a prefix says the
  carrier is newer than it, and every carrier now ends in a rule that says
  the opposite for a request it does not match. `smoke/mixed` builds a
  trait crate on this checkout against an enum crate on the last published
  version and checks the older reader's sentence, in CI. Before 1.0 the
  trait's crate and the enum's crate use the same `whichever` minor, which
  the README says now.
- An option naming nothing in the trait, `Eror = u8` or `mismatch(congruent
  = ..)` on a trait without such a method, and a `via(Variant = ..)` naming
  no variant, are reported beside the impl, which is right without them;
  before, the whole impl was withheld and every call around the enum failed
  too. `#[dispatch(default)]` on a method the trait gives no default is
  written with its sentence as its body, as a method that cannot be
  forwarded is.
- An inherent method of a dispatch struct may name any field, marked or
  not, `#[dispatch(right, call = radius)]`, and `*` is the field marked bare
  or the only field; a struct whose marked fields each name their traits
  and marks none bare is one sentence under `*`. Before, `*` took the first
  marked field silently, and only that field could be named.
- A method taking `self` by value forwarded through `deref`, or a route
  starting with `*`, to a payload that is not a `Box` is one sentence on
  the variant: a reference or a shared pointer cannot be moved out of.
  rustc said so, on the whole enum.
- The README and the crate docs say that a path in a `mismatch` expression
  written on the trait method resolves where the enum is, as the signature
  types do, so a helper beside the trait is spelled from `crate::`; a
  compile-fail case pins rustc's error for a bare name.
- An associated const of type `f32` or `f64` is checked to agree between
  the variants, as the integers, `bool`, `char` and `&str` are; comparing
  floats in a `const` is stable since Rust 1.82, within the crate's minimum.
- The carrier macro's name hashes the trait's file as a path relative to
  the crate's manifest directory, which is the same on every machine and
  names one file in the crate; before, the last two components of the path,
  so two identical `pub` traits at the same line of `foo/x/y.rs` and
  `bar/x/y.rs` collided. A file outside the manifest directory keeps the
  two components. A carrier's name changes once with this release wherever
  the two keys differ, which nothing but an expansion can see.
- The macro crate's trait path plans each trait item before writing it,
  and its selection of targets lives in a module of its own, shared with
  the inherent path; `macros/ARCHITECTURE.md` describes the shape. Nothing
  generated changed.
- `value` on an inherent method borrows a target reached through a route
  as the return type asks, as `field` does, so `via = .name, value` on a
  method returning `&str` is the arm `&inner.name`, and `field = name` is
  that spelled short; before, `value` handed the place over by value and
  rustc reported the type. The target itself, without a route, is handed
  over as before.
- A method returning `Pin<Box<dyn Future<Output = T> + ..>>`, as
  `#[async_trait]` writes one, is forwarded as it is, and when `T` is an
  overridden associated type, or a `Self` to rewrap, every arm is awaited
  in one `async move` block that is boxed and pinned again with the `Box`
  the signature names, as the attribute itself writes; before, the boxed
  return was a plain type and the override gave rustc's mismatch at the
  trait's attribute. The decision record says why the box is the hand's.

## 0.6.0 - 2026-11-06

The shapes the surveys found written by hand: a route to the target,
by-reference `TryFrom` and `into` conversions, an enum of another crate
restated, a struct's fields naming their traits, a partial block's methods
naming their variants, and tuple returns. Nothing that compiled before is
read differently, except a trait item's `#[cfg]`, which is decided in the
trait's crate now, and a trait with more than four distinct predicates on
its items, which is reported. The generated code of the benchmarked shapes
is unchanged, so the numbers stand.

- CI checks that every number the README and the guide quote is one
  `benchmarks/RESULTS.md` holds (`benchmarks/scripts/check_numbers.py`),
  without running a benchmark; the vision had listed that check as owed.
  The behaviour tests are one file per capability under `tests/behaviour/`.
- `proc-macro-crate`, which lets a crate rename its `whichever`
  dependency, sits behind the default feature `renamed-dependency`;
  without it the macro crate's dependency tree is `proc-macro2`, `quote`
  and `syn` alone, which the surveys counted as five to ten build crates
  fewer. CI checks the build without the feature and runs
  `cargo-semver-checks` against the last published version.
- Every sentence the macros write is pinned by a `tests/ui` snapshot, and
  a unit test of the macro crate reads the sources and the snapshots and
  fails when one is not; thirty sentences had none and have a case now,
  among them the empty enum, the unit struct, the route grammar and the
  block's `default` with a body. Two sentences lost a "unexpected end of
  input" prefix.
- A method returning a tuple that holds `Self`, or an overridden
  associated type, `(Self, u32)` or `(Self::Sender<T>, Self::Receiver<T>)`,
  is forwarded: the arm takes the tuple apart, converts each element and
  builds it again, as a hand writes it. The return model is a tree of
  conversions now, where it was a wrapper around a value; the shapes that
  compiled before expand as before. A tuple under an `Option` or a `Result`
  stays a limit, since its rewrap would need a closure.
- An enum another crate defines is restated once, `remote` naming the
  real one, and a trait of this crate is forwarded over it, as a
  hand-written `impl LocalTrait for other::Either<L, R>` is allowed by the
  orphan rules; the impl is written for the real path and the restatement
  emits nothing but its carrier. A restated enum gets no conversions and
  no inherent impl. activitypub_federation and Lemmy write this over
  `either::Either` by hand, and pixi over rattler's `ShellEnum`; the
  vision's "What it will not be" had refused it for a reason the compiler
  does not share, and its bullet is reworded.
- A struct's field may name the traits it is the target of,
  `#[dispatch(Rom)] rom: Cartridge` beside `#[dispatch(Interrupt)]
  signals: Signals`, and the field marked bare takes every trait not named
  elsewhere; the struct's attribute still names every trait. A trait named
  on two fields, or on a field but not on the struct, is one sentence.
  rsynko, sabicom, activityrust and russh forward different traits to
  different fields by hand, which the struct decision had named as its
  reopening condition.
- `try_from = ref` adds `TryFrom<&Enum> for &Payload` and `try_from =
  ref_mut` adds `TryFrom<&mut Enum> for &mut Payload` too, each handing the
  reference back as the error, on the enum or on one variant; hugr writes
  22 of the first by hand and sound_flow has both in its public bounds.
  `into = Target` adds `From<Enum> for Target`, each arm converting the
  payload through its own `Into<Target>`; twitch-irc, screeps and boreholeio
  write it by hand. The words `ref` and `ref_mut` are `derive_more`'s.
- A method of a partial impl block may name the variants it forwards to,
  `#[dispatch(Wep, Wpa)] fn fmt(..) { f.write_str("Others") }`, and its
  body is the arm for every other variant: a unit variant, a payload
  without an impl, a variant the enum answers differently, or two different
  variants of a `Self` argument. `#[dispatch(default)]` on a bodiless
  method leaves it to the trait's default, neither forwarded nor written.
  Nine surveyed projects wrote an inherent bridge and a one-line impl for
  this (libwifi, polars, forest, boreholeio, nostralink, derive-sql,
  mlt-core, sourmash, Lemmy); auto_impl's `keep_default_for` is the
  precedent for the second.
- An arm reaches its target through a route: `*` for what the payload
  dereferences to, which `deref` spells, `.field` or `.0` for a field,
  `.method()` for what a method returns. A route goes on a variant or a
  struct field for every trait (`#[dispatch(via = .base)]`), on one field
  of a variant with several as the marker a struct already uses
  (`ConOut(#[dispatch] LineWriter<File>, u32)`), on the trait keyed by
  variant when the route differs per trait (`via(VarChar = .0)`), or on an
  inherent method for that method alone. Where the target's type cannot be
  named the call is `<_ as Trait>::method(..)`, qualified by the trait
  alone. Inherent dispatch gains `field = name`, which reads a field of the
  target, and `value`, which makes the arm the target itself for `with` to
  wrap or an `Option<&dyn Trait>` to widen. These are the projection
  directions the surveys collected from proof-of-sql, celestia, atuin,
  sqruff, mlt-core, memcache, regexml and screeps, among others.
- The carrier macro holds the trait's signatures and nothing else: the
  default bodies, doc comments, supertraits and associated-type bounds stay
  in the trait, out of the exported macro and out of every expansion that
  reads it. The names of the methods with a default travel beside the
  signatures, since a method without a receiver is left to its default. A
  carrier written by an earlier release is read as before.
- The README and the crate docs say what was found by trial in the surveys:
  an empty partial impl block is how the impl gets a header the enum's
  attribute cannot write (the trait's own parameters, a lifetime the enum
  lacks, one instantiation, a bound), and the error for a generic trait
  dispatched without its arguments offers that block; a block in a module
  where the trait's types are in scope is the way around "the types must be
  in scope at the enum"; `with = Into::into` returns the enum at another
  lifetime; an override in a block reaches the payloads through an inherent
  `call = name` method; a payload with a hand-written `From` stays out with
  `from = false` on its variant; and what else an expansion holds, the
  `#[automatically_derived]`, the hygienic bindings and the agreement block.
- A `#[cfg]` on a trait method, associated type or const is decided in the
  trait's crate. The carrier macro is written once per combination of the
  trait's distinct predicates, each under that combination's `cfg`, with
  the items that exist under it; before, the attribute was carried onto the
  generated item and evaluated in the enum's crate, where a feature of the
  trait's crate is an unknown `cfg` and the item was compiled out or in for
  the wrong reason (fvm's `CryptoOps::verify_signature`). A trait with
  more than four distinct predicates is reported on the fifth.
- A trait method that cannot be forwarded is written with its signature
  and its sentence as its body, so the impl is complete and every call
  around it resolves; before, the whole impl was withheld and each call
  site failed too. A signature that cannot be written, a projection
  through a parameter with several bounds, still withholds the impl.
- A `mismatch` expression written on the enum may use the method's
  parameters and, for a `&self` method, `self`, as one on the trait method
  could. The two are written in different hygiene contexts, and the
  fallback arm now binds the names where the expression was written. The
  README had listed this as a limit of the carrier;
  `tests/ui/enum_mismatch_names_argument` is a behaviour test now.
- A payload that is a type parameter of the enum, or of the struct, gives
  the impl the bound a hand writes, `Raw: Trait` for `Raw(Raw)` and
  `T: Trait` for a `deref` payload `Box<T>`, `Rc<T>` or `&T`. Before, rustc
  reported the missing bound at the variant, and the way around it, an
  empty partial impl block with a bounded header, was documented nowhere;
  sux writes about 116 such blocks.
- A method taking `Self` arguments gets its fallback arm only when the
  enum has two variants to meet, or a variant under `#[cfg]`; a one-variant
  enum needs no `mismatch` and gets no arm, as a hand writes none. The
  `#[allow(unreachable_patterns)]` that every such method carried is
  written only when a `#[cfg]` variant can leave the arm unreachable.
- The benchmarks time the random layout over 64 Ki elements instead of
  1024, a sequence short enough for the branch predictor to learn, and the
  scattered collections over a single variant, so the memory table shows
  memory alone. The numbers in the README and the guide are from the new
  run: through a `Box<dyn Trait>`, a call over random variants costs 6.8 ns
  rather than 2.3 ns, and the `match` is no faster than the vtable where
  the payload methods cannot inline. The generated code is unchanged.

## 0.5.2 - 2026-11-02

A patch for the three bugs the enum_dispatch page-1 surveys filed against
0.5.0: two compile errors from the associated-type agreement check on
lifetime-generic enums, and a `#[cfg]`-gated method in a partial block that
hid the payload's override silently. The last changes what such a method
does with the `cfg` off, which was the defect; nothing else that compiled
before is read differently.

- The associated-type agreement check compiles for every lifetime-generic
  enum. Its helpers were free functions carrying the enum's generics,
  which rustc rejected with E0794 when the first payload did not mention
  the enum's lifetime (polars' `Inputs<'a>`) and with E0309 when a payload's
  `&'a T` implied a `T: 'a` the functions could not see (mrml's
  `OneOrManyIter`). They are associated functions of a local wrapper of the
  enum now, so nothing is passed to them and the implied bounds hold; a
  disagreement is still reported on the variant.
- A method a partial impl block writes under `#[cfg]` is the block's where
  the `cfg` holds and forwarded where it does not. Before, it counted as
  written in every build, so with the `cfg` off the trait's default ran and
  hid the payload's override, silently (lsm-tree). An associated type or
  const declared under `#[cfg]` is reported in one sentence, with the two
  blocks to write instead.

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
