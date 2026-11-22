# Changelog

Notable changes, written so a person can read them. Before 1.0, breaking
changes are expected and listed without apology.

## Unreleased

- `docs/partial-impl-blocks.md` and the crate docs said that a method
  written in full in a partial impl block gets `#[inline]`. Only a method
  marked to be forwarded does; one written in full is emitted as written,
  with nothing added, as the expansion goldens show.
- `docs/troubleshooting.md` has two new entries. "cannot find macro
  `Wrapper` in this scope", with rustc's note "it is a struct, not a
  macro", on a partial impl block means the struct or the enum lacks
  `#[whichever::dispatch]`. "too many leading `super` keywords" and
  "cannot find type `Policy` in module `super`" mean a `super::` path in
  the trait's signatures, met from another crate or module, and the fix
  is `crate::Policy` in the trait.

## 0.9.1 - 2026-11-17

Forwarding reaches further, and the documentation is rewritten in plainer
English. A dispatch struct now forwards a trait method without a receiver
and a generic associated type to its field's type, a mutable borrow takes
a route of its own with `via_mut`, `#[dispatch(*)]` takes a body holding
the arms the enum handles itself, a struct's field may name a trait that a
partial impl block implements, `remote` restates another crate's struct,
and `whichever::foreign::Iterator` forwards 28 more provided methods. The
tutorial, the glossary and the troubleshooting page are new, and are on
docs.rs too, and the error messages use the glossary's words. The minor
rises because a dispatch struct's provided method without a receiver now
runs the field's override where it ran the trait's default, and three
things over `foreign::Iterator` that compiled no longer do, each listed
below with its way around. There is no 0.9.0.

- The error messages use the words of `docs/glossary.md`: a method's body
  or the enum "handles" a variant where they said it "answers for" it, and
  a method is "forwarded" where it was "dispatched". So "whose body
  answers for it" is "whose body handles it", "the body answers for every
  variant" and "answers by itself" are "the body handles every variant"
  and "handles the call itself", "the variants the enum answers itself" and
  "the variants it answers" say "handles", "no variant to dispatch on" is
  "no variant to forward to", "is not dispatched by a `match`" and "can be
  dispatched" say "forwarded", and the `typetag` error says "forwarding
  them would return the payload's tag" where it said "a forwarder would
  answer with" it. `docs/structs.md` says the field's override of a
  provided method "is what runs", as the glossary does. The pages that
  quote the messages, two headings of `docs/troubleshooting.md` among them,
  and 23 snapshots changed.
- The error for an `into` target over a variant with other fields beside
  its payload no longer ends "or is left out with `from = false`": a
  variant under `#[dispatch(from = false)]` was never left out of `into`,
  which has to cover every variant, and still converts. `docs/options.md`,
  `docs/limits.md` and the crate docs say so; the behaviour test
  `conversions_by_reference_and_into` converts such a variant, and the
  snapshot `into_with_a_marked_variant` changed.
- `docs/limits.md`, `docs/options.md`, `docs/partial-impl-blocks.md`,
  `docs/troubleshooting.md` and the crate docs say that the automatic bound is a bare parameter's: a
  payload that only mentions a parameter, `Wrapped(Boxed<T>)`, gives the
  impl no bound, since what its own impl asks of `T` is not written in the
  enum, and an empty partial impl block writes the header, `impl<T: Shape>
  Shape for AnyShape<T> {}` or `where Boxed<T>: Shape`. The behaviour test
  `bounds_for_type_parameter_payloads` runs that block over an enum and a
  struct, and a recursive enum that keeps the automatic bound alone.
- `docs/options.md`, `docs/limits.md`, `docs/troubleshooting.md` and the
  crate docs say that the `From` impls reach one level: an enum held by
  another gives the outer enum `From<Inner>` and nothing for the inner
  enum's payloads, and one
  blanket impl by hand, `impl<T> From<T> for Outer where Inner: From<T>`,
  with `#[dispatch(from = false)]` on the variant, converts every leaf,
  through as many levels as are written that way. The behaviour test
  `from_impls` converts every leaf through two and through three levels.
- Before, rustc's error about what a route's method returns, such as a
  method that returns its target by value forwarded under `&self`,
  "expected `&_`, found `Parsed`", stood on the enum's or the struct's
  name, away from the route. The route's method call is now located at
  the method's name in the route, so the error lands on `via =
  .parsed()`. No new syntax: the attribute hands the method's result over
  as it is and does not borrow the temporary, `&inner.parsed()`, as a hand
  may, so such a method is written by hand. `docs/routes.md`,
  `docs/limits.md`, `docs/troubleshooting.md` and the crate docs say so; the snapshot
  `route_method_result_does_not_fit_the_receiver` changed.
- Before, a dispatch struct took nothing from its field's type that needs
  no value, though its declaration names that type: a trait method without
  a receiver and a generic associated type were refused with an enum's
  reason, "no field to forward to", an inherent method without a receiver
  likewise, and a provided method without a receiver was left to the
  trait's default, which hid the field's override of it. Where the struct
  names its target's type, the one field or the marked one, directly or
  through `deref`, a trait method without a receiver is now `<Target as
  Trait>::name(args)`, `-> Self` and `Option<Self>` built around it where
  the struct has no other field; a generic associated type is `type
  Name<..> = <Target as Trait>::Name<..> where ..;`, a parameter named
  like one of the struct's renamed, `T2`; and an inherent method without a
  receiver marked in a block is `<Target>::name(args)`, with `call` and
  `with`, so dash-num's `#[dispatch(*, call = from)] pub fn from_u64(v:
  u64) -> Self;` is `Self(<U256>::from(v))`. This changes what existing
  structs answer: a provided method without a receiver that the field's
  type overrides now answers with the override, where it answered with
  the trait's default. A method that takes a `Self` argument, a field
  holding a trait object and a field reached through a field or a method
  are left as before, a provided method to the default and a required or
  an inherent one by a sentence of its own; `field` or `value` on an
  inherent method without a receiver is a sentence. An enum is unchanged.
  `docs/structs.md`, `docs/partial-impl-blocks.md`,
  `docs/what-is-generated.md`, `docs/limits.md`, `docs/troubleshooting.md`,
  the crate docs and the docs of `#[dispatch]` say so.
  The snapshots `struct_generic_associated_type` and
  `struct_trait_method_without_receiver` now pin a field reached through a
  route, where a generic associated type says "the field of `Wrapper` is
  reached through a field or a method, so its type cannot be named to
  supply `Item`"; `struct_method_without_receiver_taking_self`,
  `struct_method_without_receiver_over_a_trait_object`,
  `struct_method_without_receiver_beside_other_fields` and
  `struct_inherent_without_receiver_reading_a_field` are new. The
  behaviour tests `newtype_delegation` and `inherent_block_on_a_struct` and
  the golden `newtype` cover a tuple struct, a named field, a marked field
  beside others, a `deref` field, a generic struct, async, unsafe and
  generic functions and an inherent constructor.
- Before, a route ending in a method call was refused for every method
  that borrows mutably, "whose result cannot be borrowed mutably; route to
  a field", though a getter that lends mutably, `fn data_mut(&mut self) ->
  &mut Data`, returns the mutable borrow itself and the hand's arm
  `inner.data_mut().to_send(version)` compiles; and the field route the
  sentence advised skipped what the getter does besides lending, such as
  confluent-kafka's `MetadataResponse::data_mut`, which drops a cache. A
  mutable borrow now takes a route of its own: `via_mut = route` on a
  variant, a variant's marked field or a struct's field, and
  `via_mut(name = route)` beside a trait, on the enum's or the struct's
  attribute or on a partial block's. A method that borrows the target
  mutably, by `&mut self`, by `self: Pin<&mut Self>` or in a `&mut Self`
  argument, follows it, and every other method follows `via`; each `Self`
  argument takes the route of its own borrow, so `fn absorb(&mut self,
  other: &Self)` is `absorb(inner.source_mut(), other.source())`. A route
  on a method's own mark, an inherent method's or a marked method's in a
  partial block, is followed whatever the method borrows, so
  `#[dispatch(*, via = .data_mut(), call = to_send)]` on a `&mut self`
  method is the hand's arm. A trait's `via(Variant = ..)` replaces both of
  the variant's own routes, and its `via_mut(Variant = ..)` given alone
  the variant's `via_mut` only. An inherent method's place arm, `value`,
  through a route ending in a method hands that method's result over as
  it is. A
  mutable borrow through a method route with no route named for it is
  still one sentence, which now names the option: ".. ends in a method
  call, which may not lend mutably; give the route a mutable borrow takes,
  `via_mut = ..`, or implement the method by hand". `via_mut` on a
  method's own mark, `via_mut = route` after a trait, and a route for a
  mutable borrow on both a variant and its field are each one sentence,
  and the option lists of the "unknown option" sentences name `via_mut`.
  `docs/routes.md`, `docs/options.md`, `docs/structs.md`,
  `docs/inherent-methods.md`, `docs/limits.md`, `docs/troubleshooting.md`,
  `docs/glossary.md`, the crate docs and the docs of `#[dispatch]` say
  so.
  The snapshots `route_method_on_mut_self`,
  `route_method_under_a_pinned_receiver_or_a_mut_argument`,
  `unknown_option`, `struct_unknown_option`, `block_unknown_option`,
  `variant_option` and `trait_named_on_a_variants_field` changed;
  `via_mut_on_the_attribute`, `via_mut_on_a_variant_and_its_field` and
  `via_mut_on_a_methods_mark` are new, and so is the behaviour test
  `mutable_routes`; the golden `routes` gained an enum over a getter pair.
- Before, the sentence for a return-position `impl Trait` on an enum said
  that "every variant returns a different opaque type", which is not true
  of variants that reach a target of one type, such as spenso's
  `DataTensor`, whose dense and sparse variants are both routed to a
  `structure` of one type; and the sentence for a `Self` argument asked
  for a `mismatch` where the trait's own default would have looked at the
  two values. The limits stay, and the sentences say what holds: "a
  return-position `impl Trait` is not dispatched by a `match`: the arms of
  a `match` must return one type, and the attribute cannot tell that the
  variants' opaque types are one; .. or implement the method by hand
  where every variant reaches one type", and, for a provided method, ".., or leave the method
  to the trait's default with `#[dispatch(default)] fn same(..);` in a
  `#[whichever::dispatch] impl Same for Kind` block", or "in this block"
  for a partial block's. `docs/limits.md`, `docs/what-is-generated.md`,
  `docs/options.md`, `docs/troubleshooting.md`, the tour in
  `docs/tutorial.md` and the crate docs say so. The snapshots `rpitit`,
  `rpitit_nested`, `impl_into_future`, `rejected_method_keeps_the_impl`
  and `two_unsupported_methods` changed;
  `self_argument_of_a_provided_method_without_mismatch` is new, and the
  behaviour test `variants_reaching_one_type` writes spenso's shape the
  way the sentences say.
- Before, a mark written under `cfg_attr`, `#[cfg_attr(feature = "x",
  dispatch)]` on a struct's field, a variant, a variant's field or a
  method of a block, was read in no build, since an attribute macro is
  handed its item before a `cfg_attr` inside it is decided: the struct was
  told to mark the field it had marked, and rustc then reported "cannot
  find attribute `dispatch`". It is now one sentence on the mark, naming
  the way around for its place: "a `dispatch` under `cfg_attr` is not
  read, since an attribute macro is handed its item before a `cfg_attr`
  inside it is decided; mark the field in every build, with a bare
  `#[whichever::dispatch]` on the struct where its own attributes are
  under `cfg_attr`", or "write the variant twice" or "write the method
  twice, under `#[cfg(..)]` with the mark and under `#[cfg(not(..))]`
  without it". The item is read as if the mark were plain, and the
  `cfg_attr` is kept without it, so the sentence stands alone.
  `docs/structs.md` shows a struct whose trait is forwarded in some builds
  only, and `docs/limits.md`, `docs/troubleshooting.md`,
  `docs/partial-impl-blocks.md`, `docs/inherent-methods.md` and the crate
  docs say so; the snapshots
  `mark_under_cfg_attr_on_a_struct_field`,
  `mark_under_cfg_attr_on_a_variant` and `mark_under_cfg_attr_on_a_method`
  and the behaviour test `marks_in_some_builds` are new.
- Before, `remote = path` on a struct's attribute restated another crate's
  struct, as it restates an enum, and nothing said so; three sentences it
  can reach called it an enum, and a restatement that left out a field of
  the real struct compiled or not by how many fields it listed, with
  rustc's "pattern requires `..` due to inaccessible fields" and a
  suggestion that is not Rust when it listed one of two. A restated
  struct now lists the fields its impls forward to, and its pattern always
  ends in `..`, as a hand's does over a struct it does not own,
  `other::Pair { left: __inner, .. }`; a field misspelt, or `-> Self`
  around a struct with fields left out, still fails against the real
  struct. The sentences speak of a struct: "`other::Solo` is another
  crate's struct, which gets no inherent impl here", the struct's own
  "`from`, `try_from` and `into` are options of an enum; a struct gets no
  conversions" for a restated struct's conversions, and `remote` on a
  block's attribute has a sentence of its own: "`remote` is an option of
  the enum's or the struct's own `#[dispatch]` attribute, which restates
  the type; a block's attribute takes the trait's options only", while
  `from`, `try_from` and `into` there keep theirs without `remote`.
  `docs/restatements.md`, under "Structs from other crates",
  `docs/structs.md`, `docs/limits.md`, the README, the crate docs and
  the docs of `#[dispatch]` say so. The
  snapshot `partial_enum_option` changed; `restated_struct_inherent_block`,
  `restated_struct_conversions` and `partial_block_remote_option` are new,
  and so are the golden and the behaviour test `restated_struct` and a
  cross-crate case in `smoke/enums`.
- Before, an option of a trait's impl written on an enum's or a struct's
  attribute that names no trait, an override `Unit = u8` or `SIDES = 7`,
  `mismatch(same = false)` or `via(A = .0)`, was accepted and did nothing;
  for a trait implemented by a partial impl block, the sentence for a
  `Self` argument even sent the user to the enum, where the option was
  read and dropped. Each is now one sentence on the option, and where the
  enum or the struct dispatches one trait from its own attributes the
  option is read beside it too, so the impl is the one meant and the
  sentence stands alone: "`mismatch(..)`
  is an option of a trait's impl, and this attribute names no trait; write
  it after the trait, `#[whichever::dispatch(Trait, mismatch(..))]`, or on
  the `#[whichever::dispatch] impl` block that implements the trait", and
  a name close to one of the enum's or the struct's own options, `form =
  false`, is "unknown option `form`; did you mean `from = ..`?". The
  sentence for a `Self` argument of a partial block's method names
  `mismatch(same = <expr>)` "on this block's attribute". `docs/limits.md`,
  `docs/options.md`, `docs/partial-impl-blocks.md`,
  `docs/troubleshooting.md`, the crate docs and the docs of `#[dispatch]`
  say so; the
  snapshots `trait_option_on_an_enum_naming_no_trait`,
  `trait_option_on_a_struct_naming_no_trait`,
  `misspelled_option_on_an_attribute_naming_no_trait` and
  `mismatch_on_the_enum_for_a_partial_block` are new.
- Before, a variant whose payload is an associated type of one of the
  enum's type parameters, `Out(S::Out)` or `Out(<S as Source>::Out)`, got
  a `From` impl that can never compile, reported by rustc as E0119 on the
  attribute against `core`'s `From<T> for T`, naming neither the variant
  nor the way around. It is now one sentence on the payload, with one
  variant as with several: "`S::Out` is an associated type of `S`, a type
  parameter of `Plain`, and may be `Plain` itself, so `From<S::Out>` would
  overlap `From<T> for T`; add `from = false` on the enum, or
  `#[dispatch(from = false)]` on this variant", and its `TryFrom` under
  `try_from` has a sentence of its own naming `#[dispatch(try_from =
  false)]`. A projection from a type that is not a type parameter,
  `<Fixed as Source>::Out`, still gets its conversions.
  `docs/limits.md`, `docs/options.md`, `docs/troubleshooting.md` and the
  crate docs say so; the snapshots
  `from_overlap_associated_type_of_a_parameter` and
  `try_from_associated_type_of_a_parameter` are new, and the behaviour
  test `from_impls` converts a concrete projection.
- Before, an `#[inline(..)]` written on a required trait method, to ask
  for it on the forwarders, stayed on the trait's own method as well,
  where rustc 1.100 warns "the `inline` attribute cannot be used on
  required trait methods", announced as a future hard error, and Clippy
  denies it. `#[dispatchable]` now keeps the attribute in the carrier macro
  for the forwarders and leaves it off a method without a body; a provided
  method keeps its own, and one written under `cfg_attr` is not carried.
  `docs/what-is-generated.md`, `docs/partial-impl-blocks.md` and the docs
  of `#[dispatchable]` say so. The golden `carried_attributes` changed; the behaviour test
  `carried_items` checks the trait under `#[deny(unused_attributes)]`.
- The tutorial, the glossary and the troubleshooting page are on docs.rs
  too, as `whichever::docs::tutorial`, `docs::glossary` and
  `docs::troubleshooting`, so they read as they were at the version you
  depend on and not as they are on `main`. The modules hold no items and
  exist only while rustdoc builds documentation, so they add nothing to
  the API. The three Markdown files now ship in the package. A link in
  them goes to the matching section of the crate docs when it is read
  there, and `scripts/check_doc_links.py`, which CI runs, fails on a link
  in the rendered docs that reaches no page or no heading.
- The documentation is rewritten in plainer English, with every fact
  kept. The README, the pages under `docs/`, the crate docs and the docs
  of the two attributes now open with the everyday case and a small
  example, and keep the fine print under headings of its own. Three
  pages are new: `docs/tutorial.md`, a guided tour whose code is the
  runnable `examples/tour.rs`; `docs/glossary.md`, the words the docs
  use, one per thing; and `docs/troubleshooting.md`, which goes from an
  error message to its cause and its fix. `docs/limits.md` gains a line
  for each limit that had a `tests/ui` case and none on the page.
  Four things the old text had wrong are corrected. The call a route
  writes: `<<Box<Square> as Deref>::Target as Shape>::area(&**inner)`
  for `deref`, `<_ as Shape>::area(&inner.base)` for a field, and never
  `<Square as Shape>::area(&inner.base)`. The variant an associated
  type or const comes from: the first that is always compiled in and
  whose target has a type to name. The cost of a cyclic layout in the
  guide, where the `match` keeps up with the uniform case only up to 8
  variants. And the options in the docs of `#[dispatch]` itself, which
  listed three of the four a variant takes and left out `into`,
  `Name = value` and the by-reference `try_from` forms.
  `scripts/check_links.py` checks that every link between the pages
  points at a file and a heading that exist, and CI runs it. No
  behaviour changed.
- Before, a trait method whose own generic parameter is named like
  something the impl gives a meaning around the method did not compile, or
  meant something else. Named like a parameter of the enum or the struct,
  or of a partial block's header, the impl declared two parameters of one
  name, rustc's E0403 (E0496 for a lifetime), on the attribute: `struct
  Tagged<I>` under `foreign::Iterator` failed at `eq<I>` and the other
  methods taking an `I`, and `enum Either<A, B>` under `foreign::Iterator`
  through a partial block's header failed on 0.8.0 too, at `fold<B, F>`.
  Named like a type the impl writes into the method, a payload's in an arm
  or an argument given to the trait, `fn pick<Square>` over a payload
  `Square` or `fn encode<Json>` under `Encoder<Json>`, the parameter stood
  for that type there. The method's parameter is now renamed as a hand
  renames it, with the first number that neither the impl nor the trait
  writes, in its declaration, its bounds, its `where` clause, its argument
  and result types and the turbofish of the arms: `fn eq<I2>(self, other:
  I2)`, `fn get<'a2>(&'a2 self, ..)`, `fn fill<const N2: usize>(..)`, `fn
  pick<Square2>`, and `fn pick<T3>` where a type `T2` is in the signature
  or among the payloads. It is renamed before the trait's parameters are
  replaced, so a method's `T` stays apart from the enum's `T` that
  `Encoder<T>` on `enum E<T>` puts in place of the trait's `F`. The trait
  method's own `mismatch` expression follows the new names,
  `#[dispatch(mismatch = T::default())]` giving `_ => T2::default()`,
  inside a macro's arguments too, `vec![T::default()]`, and where a format
  macro's format string captures a const parameter, `format!("{N}:{:N$}",
  7)`; any other string is text and stays, `String::from("{N}")` and
  `vec!["{N}"]` included, and so does the input of `stringify!` and
  `cfg!`, while a user's macro that quotes its input as text is renamed
  like code, since it cannot be told from one that runs it; the format
  string is the argument at its
  place, the arguments read as expressions, so a comma inside generic
  arguments or a closure before it does not move it. A name the impl
  writes is one that stands first in a path, which the
  parameter would stand for, so `fn pick<IntoIter>` over a payload
  `std::vec::IntoIter<u32>` keeps its name. A method a partial block
  writes or marks keeps its signature as written.
  `docs/what-is-generated.md` and `docs/options.md` say so. The behaviour
  test `method_parameters_named_like_what_the_impl_names` and the golden
  `method_parameters_named_like_what_the_impl_names` are new; the
  behaviour test `iterator_provided_methods` names its generic struct
  `Tagged<I>` again; no snapshot and no other golden changed.
- Before, two conversion options that disagree went unreported: `from =
  true` on one attribute of an enum and `from = false` on another wrote no
  `From` impl and said nothing, so the first sign was rustc's E0308 at a
  call of `E::from(payload)`, far from the attributes; and one `into`
  target given twice, on two attributes or on one, wrote the impl twice,
  rustc's E0119 with both labels on the first attribute. Each is now one
  sentence on the later occurrence: "`from = false` contradicts `from =
  true` on another attribute of `E`; the `From` impls are the enum's,
  written once whatever its attributes, so give `from` one value", in
  either order, and "`into = u32` is given twice on `E`; `From<E> for u32`
  is the enum's, written once whatever its attributes, so name a target
  once", with the impl written once. Two attributes that agree on `from`
  are fine; the `try_from` forms still add up to the widest, so `try_from
  = false` beside another form cancels nothing and is not reported; and
  different `into` targets are collected (#12). `docs/options.md`,
  `docs/limits.md` and the crate docs say so. The `tests/ui` cases
  `from_contradicts_another_attribute`,
  `into_target_twice_on_two_attributes` and
  `into_target_twice_on_one_attribute` are new, and the behaviour test
  `conversion_options_on_repeated_attributes` is extended with agreeing
  `from` values and `try_from = false` beside `try_from = true`.
- Before, an item of a partial impl block that cannot be read was
  reported in every build, whatever `#[cfg]` it carried, and the impl was
  withheld with it, so every call of the trait's methods was rustc's E0599
  as well: `#[cfg(any())] #[dispatch(default)] fn name(&self) -> &'static
  str { "x" }` failed every build with "`name` is left to the trait's
  default, so it takes no body", where a hand's method under the same
  `cfg` is simply absent. The same held for a method without a body and
  without a mark, a mark that does not parse or carries an inherent
  method's option, and a second mark on one method. Such an item is now in
  the block only where its `cfg` holds, as a macro invocation already is,
  below: where the `cfg` does not hold, the block is read without it and
  the method is forwarded, and where it holds, each such item has its
  sentence, where only the first was reported. Without a `#[cfg]` nothing
  changed
  (#22). `docs/limits.md`, `docs/partial-impl-blocks.md` and the crate
  docs say so. The `tests/ui` case `block_items_under_a_cfg_that_holds` is
  new, and the behaviour test `conditional_methods_in_a_partial_block` is
  extended; no other snapshot and no golden changed.
- Before, an inherent method of a dispatch struct that named two fields,
  `#[dispatch(square, circle)] pub fn side(&self) -> u32;`, compiled into a
  `match` with two arms of which the second is never reached, so the
  method was silently forwarded to the first field alone. It is now one
  sentence: "`side` names two fields, `square` and `circle`, and a struct
  forwards a method to one, since every value holds them all; name one,
  and write a method that needs both by hand". The `tests/ui` case
  `struct_method_naming_two_fields` is new.
- Before, a struct's field could name only a trait the struct's own
  attribute dispatches, so a trait whose impl needs a header the attribute
  cannot write had no way to a field other than the bare one: the field's
  `#[dispatch(Catalog)]` beside `#[whichever::dispatch] impl<S: Sorts, C:
  Catalog<Extractor = S::Extractor>, N> Catalog for Env<S, C, N> {}` read
  "`Catalog` is named on `catalog` but not on the struct", naming it on the
  struct as well was the second impl, and a method of the block marked
  `#[dispatch(catalog)]` read "`Env` has no field `catalog`", which was
  false. A trait's impl has one home, the struct's attribute or a partial
  impl block, and the struct now forwards a trait to the field that names
  it whichever home writes the impl; the record "A field's trait may be a
  partial block's" says why. A field names a trait as its home spells it,
  generic arguments included, `#[dispatch(Encoder<F>)]` for `impl<F, ..>
  Encoder<F> for Env<..> {}`, and from `crate`, `self`, `super` or the
  root where the home's path begins there. Before, a field's option list
  was read by its first identifier, so `#[dispatch(crate::traits::Catalog)]`
  on a field was "expected identifier, found keyword `crate`", in 0.8.0
  as well, and a home spelling its trait so had no field that could name
  it. The struct does not see its blocks, so a
  field's mark that no home claims is no longer reported; the trait's
  first use says that the impl is missing. One name spelled two ways is
  reported instead, at either home: "`Catalog` on `catalog` and
  `traits::Catalog` on the struct spell one trait's name two ways", and
  the same for `Encoder` beside `Encoder<F>`; spell it as the home does.
  The impl is then written for the field that was meant, so the sentence
  stands alone: no rustc error on a bare field that does not implement
  the trait, and no impl withheld at a block, which would fail every call.
  A block's method naming another field than the trait's now says "`named`
  is a field of `Env`, but this trait is forwarded to `catalog`, ...; name
  the trait on `named`, `#[dispatch(Trait)]`, to forward it there", and a
  trait's route keyed by another field says the same. `via = route` after
  a trait, which read as an override named `via`, says where a route
  goes. rsynko 0.1.2's `LawEnv` is the shape (#16). The behaviour test
  `per_trait_fields` is extended; the `tests/ui` case
  `field_names_an_undispatched_trait` is gone with its sentence, and
  `struct_trait_spelt_two_ways`, `block_trait_spelt_two_ways_by_path`,
  `block_trait_spelt_two_ways_by_arguments`,
  `block_method_naming_another_field`, `route_keyed_by_another_field`,
  `via_after_a_trait` and `struct_trait_spelt_two_ways_from_self` are new.
- Before, several sentences about a dispatch struct spoke of variants and
  the enum: a struct whose field is reached through a method, under a
  trait with an associated type, read "every variant of `Wrapped` is under
  `#[cfg]` or reached through a field or a method, ...; fix it on the enum
  with `Unit = ..`". A sentence is now written in the words of the item the
  attribute is on, so the same struct reads "the field of `Wrapped` is
  reached through a field or a method, so its type cannot be named to
  supply `Unit`; fix it on the struct with `Unit = ..`". The same holds
  for a generic associated type ("which only the struct can declare"), a
  trait method without a receiver ("no field to forward to"), an unknown
  option (a struct's list has no conversion and no `mismatch`), an
  override naming nothing in the trait (a struct's other option is a
  route, `via(field = route)`), a method's mark that names nothing or
  names `*` beside a field, a body under a forwarded mark ("forwarded to
  the struct's field"), a bodiless method without a mark in an inherent or
  a partial block, a return-type limit on a method with a body ("the body
  answers by itself"), and a trait's route keyed by a name the struct has
  no field of. A partial impl block's own attribute is read before the
  enum or the struct is known, so its sentences name neither: a generic
  associated type there is one "which the block declares and no option
  gives", an unknown option
  lists the trait's options, and a conversion option is "an option of an
  enum's own `#[dispatch]` attribute; a block's attribute takes the
  trait's options only". Three sentences that spoke of variants on either
  item now speak of neither: a second mark on a method, arguments on an
  inherent block's attribute, and an inherent option on a trait method's
  mark. Only the words changed. The `tests/ui` cases
  `struct_field_through_a_method_supplies_no_type`,
  `struct_generic_associated_type`, `struct_gat_override_option`,
  `gat_override_on_a_blocks_attribute`,
  `struct_trait_method_without_receiver`, `struct_unknown_option`,
  `block_unknown_option`, `struct_override_naming_nothing`,
  `struct_method_marks_naming_no_field`,
  `struct_inherent_bodiless_without_attribute`,
  `struct_block_bodiless_without_attribute`,
  `struct_lift_over_a_wrapped_self` and `struct_route_through_no_field`
  are new; `block_default_marked_twice`,
  `block_method_under_a_cfg_that_holds`, `block_method_with_call`,
  `impl_attribute_error_keeps_block`,
  `impl_attribute_error_keeps_own_arms_under_a_boxed_receiver`,
  `inherent_all_with_body_on_a_struct`, `partial_enum_option`,
  `two_inherent_attributes` and `unknown_option_names` moved with the
  words.
- Before, `whichever::foreign::Iterator` forwarded `next`, `size_hint`,
  `count`, `last`, `nth` and `fold`, and every other provided method ran
  the trait's default on the enum, so a payload's override was hidden:
  through `#[dispatch(whichever::foreign::Iterator)]`, a payload's own
  `any` was never called. It now forwards every provided method a `match`
  can forward and that is stable on 1.88, 28 more: `for_each`, `collect`,
  `partition`, `reduce`, `all`, `any`, `find`, `find_map`, `position`,
  `max`, `min`, `max_by_key`, `max_by`, `min_by_key`, `min_by`, `sum`,
  `product`, `cmp`, `partial_cmp`, `eq`, `ne`, `lt`, `le`, `gt`, `ge`,
  `is_sorted`, `is_sorted_by` and `is_sorted_by_key`. `core`'s slice
  iterators answer `any`, `all`, `find`, `find_map` and `position`
  themselves, a range answers `max`, `min` and `is_sorted`, and `collect`
  reaches the payload's own, so a `vec::IntoIter` collected into a `Vec`
  reuses its allocation; the defaults of `all`, `any`, `find` and
  `position` run through `try_fold`, which cannot be restated, and matched
  the enum once per item where they are now one `match`. Left out:
  `unzip`, which a `match` cannot forward; `rposition`, whose bound holds
  only for an enum that is also double-ended and exact-size; the methods
  unstable on 1.88, such as `try_fold`; and the adapters and `by_ref`, as
  before. `DoubleEndedIterator` and `ExactSizeIterator` were complete
  already. `Error` still leaves out `description` and `cause`, which are
  deprecated, so that a block that answers `source` itself need not answer
  them. Three things that compiled no longer do, each with its way around:
  - Every added method requires `Self: Sized`, so a `deref` variant
    holding a trait object, `#[dispatch(deref)] Dyn(Box<dyn Iterator<Item
    = u32>>)`, is refused for each of them, 31 sentences where there were
    3. `Box<dyn Iterator>` is itself an `Iterator`: hold it without
    `deref`, and every method is forwarded to the box, with no partial
    block. The same holds for a `Box<dyn DoubleEndedIterator>` under
    `DoubleEndedIterator` and a `Box<dyn ExactSizeIterator>` under
    `ExactSizeIterator`. The sentence now offers this way beside
    `#[dispatch(default)]`: "hold `Box<dyn Iterator<Item = u32>>` without
    `deref` where it implements the trait itself".
  - A partial block over `foreign::Iterator` forwards every method it
    leaves alone, so one whose enum holds a variant no method can be
    forwarded to has 34 methods to answer or leave to the default, not 6.
  - An override, `Item = Wide`, converts what a method returns and hands
    an argument to the payload as it is, so over payloads whose items
    differ, a `Range<u32>` and a `Range<u64>`, rustc reports every method
    that takes a closure over `Self::Item`, where before it reported
    `fold` alone (#23). An override of the payloads' own type, `Item =
    u8` over payloads whose `Item` is `u8`, compiles as before.

  The way around all three is a restatement of `Iterator` in your crate
  listing the methods to forward, `next` and `size_hint`, since the rest
  take the real trait's default; `docs/restatements.md` shows it, and
  `docs/options.md` and `docs/limits.md` say what an override converts.
  The behaviour tests `iterator_provided_methods` and
  `item_override_through_a_narrower_restatement` are new;
  `trait_objects_in_deref_variants` now shows `#[dispatch(default)]` over
  such a restatement and the box held without `deref`, and
  `double_ended_and_exact_size_iterators` the boxes held without `deref`.
  The golden `foreign` shows `Iterator`, and the snapshots
  `dyn_payload_under_sized_method` and `trait_object_beside_a_plain_variant`
  moved with the sentence.
- Before, a method of an inherent `#[dispatch]` impl block that cannot be
  forwarded was reported in every build, whatever `#[cfg]` it carried:
  `#[cfg(feature = "off")] #[dispatch(*)] pub fn make(side: f64) -> Self;`
  failed the build without the feature with "`make` has no receiver, so
  there is no variant to forward to", where a hand's method under the same
  `cfg` is simply absent. The same held for a mark that cannot be read,
  `#[dispatch(*, Circle)]`, a second mark, a variant the enum lacks, and a
  method without a body or a mark, on an enum or a struct. The sentence is
  now under the method's own `#[cfg]`, so a build that compiles the method
  out reports nothing, and one where the `cfg` holds reports as before.
  Tokens in the block that are no method are still reported in every
  build. dash-num 0.3.0-beta's constructors behind a feature are the shape.
  `docs/limits.md`, `docs/inherent-methods.md` and the crate docs say so.
  The `tests/ui` case `block_method_under_a_cfg_that_holds` is new, and
  the behaviour test `cfg_on_variants_and_methods` is extended; no other
  snapshot and no golden changed.
- Before, a partial impl block that leaves a trait method with a `Self`
  argument to the plain forwarding, `fn f(&self, other: Self, n: u64)`,
  over an enum whose only variant in the build cannot be forwarded to,
  `B(P1, u64)` beside `#[cfg(any())] A(P0)`, reported the variant's
  sentence and, beside it, rustc's unused variable: `n`. The arm that
  stands in for the variant was left out where a `Self` argument is
  matched beside `self`, and the arm for two different variants does not
  read `n`. That arm is now written beside the `Self` argument too, `(E::B
  { .. }, ..) => { let _ = (&n,); ::core::unreachable!() }`, and reads the
  arguments the `match` does not hold, so the sentence stands alone. The
  `tests/ui` case
  `block_over_an_unforwardable_variant_beside_a_self_argument` is new; no
  other snapshot and no golden changed.
- Before, an inherent method taking `self` by value and forwarded by name
  to a `deref` variant holding a `&'a T`, an `Rc`, an `Arc` or a `&mut T`,
  `#[dispatch(*)] pub fn base(self) -> &'a Base;` over
  `#[dispatch(deref)] Int(&'a Int)`, was refused: "`base` takes `self` by
  value, and `Int` dereferences a `&'a Int`, which a value cannot be moved
  out of; hold a `Box`, or give it a body". Nothing moves there when the
  payload's method takes `&self`, and the hand's arm beside it compiles.
  This was a regression: 0.5.2 compiled the shape, 0.6.0 gave rustc's
  E0308, and 0.7.0 to 0.8.0 gave the sentence. The arm is now the hand's,
  `Self::Int(inner) => (*inner).base()`, under `*`, a named list and `call
  = name` alike, and rustc judges it: a payload method taking `&self`
  borrows what the variant points at, one that consumes its receiver is
  rustc's E0507, and a reference returned out of an `Rc` the arm drops is
  rustc's E0515, as in a hand's arm. A trait method taking `self` or a
  `Self` argument by value over such a variant, and an inherent `value` or
  `field` the return type does not borrow, are reported as before.
  cameleon-genapi 0.1.14's `node_base_precise` is the shape.
  `docs/limits.md`, `docs/inherent-methods.md` and the crate docs say so.
  The behaviour test `inherent_methods_on_dereferenced_payloads` is
  extended; no golden and no `tests/ui` snapshot changed.
- Before, `#[dispatch(*)]` with a body was an error, and the only way to
  forward to every variant but a few was to name every forwarded variant
  on the method, with the body as the arm for the rest: a list as long as
  the `match`, which a new variant left off passed silently. Under `*`, a
  method of a partial impl block or of an inherent block may now have a
  body that is one `match self` holding the arms the enum answers itself,
  `Self::Unknown(kind, data) => ..`, and every variant those arms do not
  name is forwarded. The generated `match` is the forwarding arms in the
  enum's order, then the arms as written, with no wildcard, so a variant
  added later is forwarded, and one that holds no payload and has no arm
  is reported on the method with the arm to write: "write its arm in the
  method's body, `match self { Self::Empty => .. }`", where the sentence
  said "name the variants with a payload and give the method a body for
  the rest". A method that can take no arm of its own, a trait method with
  a `Self` argument or one over an enum whose every variant is under
  `#[cfg]`, keeps that old way around, and an arm there, like an arm that
  names no variant, is one sentence. No build changes meaning: `*` with a
  body was an error, and a body under a named list is still the arm for
  every other variant. The golden and the behaviour test
  `own_arms_under_star` are new, and so are the `tests/ui` cases
  `own_arm_naming_no_variant`, `own_arms_beside_a_self_argument` and
  `own_arms_over_variants_all_under_cfg` for the new sentences;
  `own_arms_leaving_a_variant_without_a_payload` for the arm named as the
  way around, and `star_over_variants_all_under_cfg_without_a_payload` for
  the list kept where no arm can go;
  `own_arms_over_a_variant_the_method_refuses`,
  `own_arms_over_a_variant_that_cannot_be_rebuilt` and
  `own_arms_beside_a_self_return_the_lift_cannot_wrap` for the ways
  around a refusal offers beside arms; `inherent_all_with_body_on_a_struct`
  and `inherent_list_of_every_variant_with_body` for a body that still
  never runs; and `impl_attribute_error_keeps_own_arms_under_a_boxed_receiver`
  for a rejected block that keeps such a method. The snapshots
  `inherent_all_with_body`, `inherent_all_with_unforwardable` and
  `rejected_variant_keeps_enum` moved with the reworded sentences;
  `star_over_variants_without_a_payload_beside_a_self_argument` did not,
  and pins the old way around beside a `Self` argument.
- Before, the one field of a tuple struct was matched `Wrapped(inner, ..)`,
  and the one named field of a struct or a variant `Named { field: inner,
  .. }`, a rest pattern with nothing to skip. They are now matched as a
  hand writes them, `Wrapped(inner)` and `Named { field: inner }`; a field
  beside others keeps its `..`. This is what `cargo expand` shows and
  changes nothing at run time. The goldens `generic_payloads`,
  `named_field_variant`, `newtype` and `opaque_through_a_struct` moved; no
  snapshot changed.
- Before, an unnamed marked field that is the last of its variant or
  struct was matched with a rest pattern that had nothing to skip,
  `Last(_, inner, ..)` for `Last(u64, #[dispatch] Square)`. It is now
  matched `Last(_, inner)`, and `struct Labelled(u64, #[dispatch] Square)`
  `Labelled(_, inner)`; a field with others after it keeps its `..`, as
  does a named field beside others. This is what `cargo expand` shows and
  changes nothing at run time. The golden `marked_last_field` is new; no
  other golden and no snapshot changed.
- Before, a variant whose one field is named and marked, `Named {
  #[dispatch] shape: Square }`, was read as a field beside others, so a
  method returning `Self` was refused with "cannot be built around the
  other fields of `Named`", fields the variant does not have. It is now
  rebuilt by its braces, `Named { shape: value }`, as a struct with one
  named field is. Under `Option<Self>` or `Result<Self, E>`, which a path
  rebuilds and no path builds such a variant, the sentence names the tuple
  variant as the way around: "write the variant as `Named(Square)`". The
  variant still gets no `From` or `TryFrom`. The `tests/ui` case
  `self_in_an_option_over_a_variants_named_field`, the golden
  `named_field_variant` and the behaviour test
  `rebuilt_named_field_variants` are new; no other snapshot and no golden
  changed.
- Before, a trait named on a variant's field, `Node { #[dispatch(Shape)]
  data: Data, id: u32 }`, was accepted and ignored, since a variant's field
  was read with a struct field's grammar. It is now one sentence: the
  options of a variant's field are `deref` and `via = route`, and a trait
  takes its own route through the variant on the enum, `via(Variant =
  route)`. `from` and `try_from` on a variant's field are reported the same
  way. The `tests/ui` case `trait_named_on_a_variants_field` is new.
- Before, a macro invocation in a partial impl block was dropped without a
  word, and the forwarding stood in place of the items it would have
  written. It is now one sentence on the invocation; write the items out
  in the block. One under `#[cfg]` is reported only where the `cfg` holds:
  where it does not, the invocation is not in the block, as it is not in a
  hand's, and the block is read without it, so a method the macro would
  have written is forwarded. The `tests/ui` cases
  `macro_in_a_partial_block` and
  `macro_in_a_partial_block_under_a_cfg_that_holds` are new, and the
  behaviour test `conditional_methods_in_a_partial_block` is extended.
- Before, a second `#[dispatch(..)]` after `#[dispatch(default)]` on a
  method of a partial impl block was ignored. It is now reported, "one
  `#[dispatch(..)]` per method", as a second mark in any other order
  already was. The `tests/ui` case `block_default_marked_twice` is new.
- Before, when an item withheld the impl, such as an associated type the
  block declares under `#[cfg]`, its error was reported alone, and an
  option naming nothing in the trait or a route through no variant showed
  only in the next build. They are now reported together. An error that
  stops the partial block from being read, such as the macro invocation
  above, is still reported alone. The `tests/ui` case
  `withheld_impl_keeps_the_errors_beside_it` is new.
- The macro crate's modules were reorganised around the concepts they
  hold; `macros/ARCHITECTURE.md` describes the new shape. No expansion
  changed.
- `docs/options.md` and the crate docs say what the conversion options
  mean on an enum that repeats the attribute: the conversions are the
  enum's and written once, `from = false` on any attribute leaves the
  `From` impls out, the widest `try_from` form applies, and the `into`
  targets of every attribute are collected. The behaviour test
  `conversion_options_on_repeated_attributes` is new.
- `docs/limits.md` and the crate docs say how an enum keeps its imports
  short when the trait is in another crate: a glob import of the trait's
  crate at the enum, or an empty partial impl block per trait in a module
  that holds the glob. The attribute does not write the glob itself, since
  around the impl it would shadow the enum module's own names.

## 0.8.0 - 2026-11-15

The fixes of the surveys of acadrust 0.6.8 and mp4-atom 0.17.1, and the
first fixes the conformance matrix found. The matrix is new in the
workspace, under `conformance/`: a model of what the attributes support,
one small program per selected combination of features, each judged
against a hand-written twin, run by CI together with the mutants that
measure it. The minor rises because three things that compiled are read
differently, each listed below with its way around: `value` on a plain
payload or a struct's field whose method hides a reference behind a type
alias is read by value; a dispatch struct with one named field no longer
forwards a method returning `Option<Self>` or `Result<Self, E>`; and the
associated consts of an enum with lifetime parameters alone are checked to
agree. The benchmark suite was rerun for this release; the numbers the README
and the guide quote are from that run, and the generated code of the
benchmarked shapes compiles to the hand-written impl's instructions, as
before.

Three fixes and a sentence from the survey of 2026-11-13 of acadrust
0.6.8: the first fix found by the survey, the second, the sentence and the
third while probing the first.

- Before, an inherent method with `value` made the arm the pattern's
  binding as it is, a `&Handle` under `&self`, even where the method
  returns `Handle`: on a plain payload, `Plain(Handle)`, on the marked
  field of a variant with several, `Unknown { name: String, #[dispatch]
  handle: Handle }`, and on a dispatch struct's field, `struct Slot {
  #[dispatch] handle: Handle, label: String }`. The arm did not compile,
  rustc's E0308, expected `Handle`, found `&Handle`, and beside a routed
  variant, which already read a value, the arms did not agree. acadrust's
  `ObjectType::intrinsic_handle` reads a `handle` over 35 tuple variants
  and such a struct variant. Every target is now handed over as the return
  type asks, as a field is: under a `&self`, `&mut self` or pinned
  receiver `#[dispatch(*, value)]` on `fn handle(&self) -> Handle` is
  `*inner`, `Some(*inner)` under `with = Some` for `-> Option<Handle>` and
  `Ok(*inner)` under `with = Ok` for `-> Result<Handle, E>`. A method that
  returns a reference, `-> &Handle`, `-> &mut Handle` or `-> Option<&Handle>`
  under `with = Some`, still gets `inner`; a lift other than `Some` or
  `Ok`, a raw pointer return and an `impl Trait` return on a dispatch
  struct are still handed the target as it stands, the binding for a
  target the pattern binds and a place reached through `via` or `field` by
  value; `self` by value and an `async` method are as before. Parentheses
  around the return type, or the group a `macro_rules!` fragment leaves,
  are looked through. One thing that compiled before no longer does:
  `value` on a plain payload or a struct's field whose method hides a
  reference behind a type alias, `type CircleRef<'a> = &'a Circle;` and
  `fn circle(&self) -> CircleRef<'_>`, since the return type is read as
  written and the alias now reads as a value, as `field` and routes always
  read it; spell the reference in the signature, `-> &Circle`. The
  expansion goldens `marked_field_by_value` and `payload_and_field_by_value`
  are new and show the arms; no other golden and no `tests/ui` snapshot
  changed.
- Before, `value` or `field = name` on a `deref` variant under `self:
  Box<Self>` dereferenced the payload once too often: `#[dispatch(*,
  value)]` on `fn into_handle(self: Box<Self>) -> Handle` over
  `#[dispatch(deref)] Boxed(Box<Handle>)` did not compile, rustc's E0614,
  type `Handle` cannot be dereferenced. The enum is matched by value
  there, so the arm is now `*inner` for `value` and `(*inner).name` for
  `field = name`, as with `self` by value. Every other receiver is
  unchanged, and a variant without a route was already right. No golden
  and no `tests/ui` snapshot changed.
- Before, `value` or `field = name` under `self: Box<Self>` on a `deref`
  variant holding an `Rc`, or any pointer but a `Box`, reached rustc as
  E0507, cannot move out of an `Rc`, on the enum's attribute. It is now
  the sentence, on the payload, as under `self` by value: "`into_name`
  takes `self: Box<Self>`, and `Shared` dereferences a `Rc<Name>`, which
  a value cannot be moved out of; hold a `Box`, or give it a body". A
  place the return type borrows, `-> &'a Name` over `Borrowed(&'a Name)`,
  moves nothing and is not reported. The `tests/ui` case
  `place_moves_through_a_shared_pointer_under_a_boxed_receiver` is new; no
  other snapshot and no golden changed.
- Before, a sentence spelled a type or a path with every space taken
  out, so a payload `&'a Name` read `&'aName`, and `&'a mut Name` and
  `dyn Named + 'a` likewise. Sentences now spell them as written, `&'a
  Name`, `Rc<dyn Named + 'a>`, `Result<A, B>`. The `tests/ui` case
  `sentence_spells_a_type_with_its_spaces` is new; no existing snapshot
  changed, since none held such a type.

Fixes the conformance matrix found, and one the survey of 2026-11-13 of
mp4-atom 0.17.1 found that the matrix showed to be wider.

- Before, a rustc error inside an arm was reported on a span from the
  trait's `#[whichever::dispatchable]` to the enum's name, or from the
  enum's `#[whichever::dispatch]` on an inherent method, where a hand's
  impl gets it on its own code. A route ending in a method call whose
  result does not fit the receiver, `via = .copied()` under `&self`, is
  rustc's E0308 or E0277, and an inherent `with = function` handed a place
  it cannot take is E0308. The arm's bindings were located at the
  attribute that wrote the trait's carrier macro while the rest of the arm
  was at the enum, and an expression holding both stretched between them.
  They are now located where the arm is, so each of these errors is on
  the enum's name, or on the inherent method's name; the errors themselves
  are rustc's, for forwardings no hand can write either. A `&mut self`
  method through an `Rc`, which was rustc's E0596 there too, is now a
  sentence, below. The `tests/ui` case
  `route_method_result_does_not_fit_the_receiver` is new; no other
  snapshot and no golden changed.
- Before, a method that could not be forwarded to one variant was reported
  in every build, whatever `#[cfg]` the variant carried, and named the
  first such variant alone. A `deref` variant under `#[cfg(test)]` beside
  `fn grown(&self) -> Self` failed the build that leaves the variant out
  with "`grown` returns `Self`, which cannot be wrapped back into the
  dereferenced payload of `Boxed`", where a hand's arm under the same
  `cfg` compiles; the same held for a `Self` return through a route or
  beside other fields, a route ending in a method call under `&mut self`,
  `self: Box<Self>` on a variant that is not a `deref` over a `Box`, and a
  move out of a shared pointer. The sentence is now the result of that
  variant's arm, under the variant's own `#[cfg]`: a build without the
  variant forwards the rest, a build with it reports as before, and each
  variant the method cannot reach is named. A second effect of the same
  change: `#[dispatch(*)]` in a partial block over a variant without a
  payload, for a method taking a `Self` argument, was answered by its
  sentence and by rustc's E0308 on the enum's name, and is now answered by
  the sentence alone. The `tests/ui` cases
  `deref_with_self_return_under_a_cfg_that_holds` and
  `star_over_variants_without_a_payload_beside_a_self_argument` are new;
  the snapshot `route_moves_through_a_shared_pointer` gains the sentence
  for its second variant, `Borrowed`, which it did not name before; no
  other snapshot and no golden changed.
- Before, a partial impl block forwarding a method that takes a `Self`
  argument by value or by `&mut`, `fn merge(self, other: Self, by: u64)`
  under `mismatch(merge = by)`, warned in the user's crate, unused
  variable: `other`: the arm for two different variants bound every such
  argument again, so that the `mismatch` might use it, whether it did or
  not. The enum's own attribute hid the warning, since its impl is
  automatically derived and a partial block's is not. The argument is now
  bound again only where the `mismatch` names it, a name a format string
  captures, `format!("{other:?}")`, included. Beside it, a partial block
  over an enum whose only variant in the build holds no payload reported
  the variant's sentence and, with it, unused variable on each of the
  method's arguments; the sentence now stands alone. The `tests/ui` case
  `block_over_a_variant_without_a_payload_alone_in_the_build` is new; no
  other snapshot and no golden changed.
- Before, a partial impl block whose mark sat under the trait method's own
  `#[cfg]`, `#[cfg(test)] #[dispatch(default)] fn extra(&self) -> u8;` for
  a trait that declares `extra` under `#[cfg(test)]`, reported a variant no
  method can forward to, `Other(u8, u8)`, in the build where the `cfg`
  holds, so `cargo build` passed and `cargo test` failed. The mark leaves
  the plain forwarding only the builds where the `cfg` does not hold, in
  which the trait has no such method, and the variant was reported as
  though the method were forwarded to every variant. A mark naming
  variants, `#[dispatch(*)]` and a method the block writes, each under the
  trait method's `#[cfg]`, did the same; mp4-atom 0.17.1's `Encode`, with a
  `#[cfg(test)]` provided method, is the shape. The variant is now
  reported in the builds in which some method is forwarded to every
  variant and in no other, so a mark under a `cfg` the trait method does
  not carry still leaves it reported where the `cfg` does not hold.
  `docs/partial-impl-blocks.md` says so. The `tests/ui` case
  `conditional_mark_leaves_builds_to_the_forwarding` is new; no other
  snapshot and no golden changed.
- Before, a generic trait whose parameter's one bound constrains an
  associated type, `trait Encoder<F: Format<Value = u64>>` with a method
  taking `F::Value`, could not be dispatched: the projection was
  qualified with the bound as written, `<Json as Format<Value =
  u64>>::Value`, and rustc rejected it with E0229, associated item
  constraints are not allowed here, on the trait's bound. The projection
  is now qualified with the bound's generic arguments and without its
  constraints, `<Json as Format>::Value`, and `<Json as
  Format<Text>>::Value` for `F: Format<Text, Value = u64>`, which name the
  same type. `docs/options.md` says so. No golden and no `tests/ui`
  snapshot changed.
- Before, three sentences named a receiver the method does not take, and
  one gave advice that would not compile. `fn merge(&self, other: Self)`
  over `#[dispatch(deref)] Shared(Rc<Square>)` was told that "`merge`
  takes `self` by value", where `other` is what would move; it is now
  "`merge` takes `other` by value, and `Shared` dereferences a
  `Rc<Square>`, which a value cannot be moved out of". A route ending in
  a method call under `self: Pin<&mut Self>`, or with a `&mut Self`
  argument, was told that the method "takes `&mut self`"; it now names
  `self: Pin<&mut Self>`, or `other: &mut Self`. `fn finish(self:
  Box<Self>)` over a `deref` variant holding an `Rc<Square>` was told to
  "hold `Box<Rc<Square>>` in a `#[dispatch(deref)]` variant", which
  `Rc<Square>` does not implement the trait to make work; it is now
  "`finish` takes `self: Box<Self>`, and `Shared` dereferences a
  `Rc<Square>`, which is no `Box` to hand over; hold a `Box`, or
  implement the method by hand". Beside them, a method that names its
  variants and has a body for the rest was told to "give it a body" for a
  variant it cannot be forwarded to; it is now offered that body, "or
  leave `Borrowed` to the body", and a method without one keeps its
  sentence. `docs/limits.md` says so. The `tests/ui` cases
  `box_self_on_a_shared_pointer`,
  `self_argument_moves_through_a_shared_pointer` and
  `route_method_under_a_pinned_receiver_or_a_mut_argument` are new; the
  snapshot `sentence_spells_a_type_with_its_spaces` moved, each of its
  four sentences now ending "or leave `Borrowed` to the body" with its
  own variant; no golden changed.
- Before, an inherent method taking `self` by value whose arm is a place
  the return type borrows, `#[dispatch(*, value)] pub fn name(self) ->
  &'a Name;` over `#[dispatch(deref)] Borrowed(&'a Name)`, was refused:
  "`name` takes `self` by value, and `Borrowed` dereferences a `&'a
  Name`, which a value cannot be moved out of". Nothing moves there, and
  the same method under `self: Box<Self>` already compiled. A place under
  `self` is now refused only where the return type takes it by value, so
  the arm hands over the reference the variant holds. A method call under
  `self` is refused as before. `docs/inherent-methods.md` and
  `docs/limits.md` say so. No golden and no `tests/ui` snapshot changed.
- A `mismatch` on the enum naming a method the trait declares under a
  `#[cfg]` that does not hold, `mismatch(same = false)` for a `same` under
  a feature that is off, is still reported, as a name the trait lacks,
  since that build's trait has no such method; the enum built only with
  the feature on. The sentence now ends with the way around it: "a method
  under a `#[cfg]` that does not hold in this build takes its mismatch on
  the trait method, `#[dispatch(mismatch = ..)]`", which travels under the
  method's `cfg`. The decision record "An option names an item of every
  build" weighs the alternative; `docs/limits.md` and `docs/options.md`
  say so. The `tests/ui` case `mismatch_for_a_method_under_a_cfg` is new,
  with both forms of the hint; the snapshots `unknown_option_names` and
  `unknown_override_keeps_the_impl` gained the clause; no golden changed.
- Before, a method that borrows its target mutably, forwarded to a `deref`
  variant over an `Rc`, an `Arc` or a `&T`, reached rustc as E0596,
  cannot borrow data in an `Rc` as mutable, in an arm the user did not
  write, where a move out of the same pointer is a sentence. It is now one
  sentence on the payload: "`grow` takes `&mut self`, and `Counted`
  dereferences a `Rc<Square>`, which cannot be borrowed mutably; hold a
  `Box` or a `&mut`, or implement the method by hand", naming `self:
  Pin<&mut Self>` or `other: &mut Self` where that is what borrows, and
  "`name` returns a `&mut`" for an inherent method whose place is
  returned so. The three pointers are recognised by name, as for a move,
  and any other is left to rustc. A shared borrow, `#[dispatch(*, value)]
  fn name(&mut self) -> &Name;` through an `Rc`, still forwards. The
  decision record "A borrow a shared pointer cannot lend is a sentence"
  weighs it; `docs/limits.md` and `docs/routes.md` say so. The `tests/ui`
  cases `mutable_argument_through_a_shared_pointer` and
  `mutable_place_through_a_shared_pointer` are new, and the snapshot
  `mutable_borrow_through_a_shared_pointer`, which held E0596 on the
  enum's name, now holds the sentence; no golden changed.
- A dispatch struct whose one field is named, `struct Named { shape:
  Square }`, no longer forwards a method returning `Option<Self>` or
  `Result<Self, E>`. Since 0.4.0 it rebuilt the value with a closure,
  `.map(|value| Named { shape: value })`, the one closure the generated
  code held, where the docs say it holds none; `.map` takes a path, and no
  path builds such a struct. It is now one sentence on the field:
  "`maybe_grown` returns `Self` inside an `Option` or a `Result`, where a path rebuilds
  the value, and `Named` has a named field, which no path builds; make it
  a tuple struct, `struct Named(Square);`, or write the method in a
  `#[whichever::dispatch] impl` block". This breaks a struct that
  compiled: one named field and a dispatched method returning
  `Option<Self>` or `Result<Self, E>`; make it a tuple struct, which
  forwards all three as before, or write the method in a partial block.
  `-> Self` on the named struct is rebuilt by its braces as before. The
  decision record "A named field is not rebuilt under a wrapper" weighs
  it; `docs/structs.md`, `docs/limits.md` and the `Self` row of
  `docs/what-is-generated.md` say so. The `tests/ui` case
  `named_field_under_a_wrapper` is new; no other snapshot and no golden
  changed.
- Before, two sentences named the wrong item. A method returning `Self`
  over the marked field of a variant with several, `Tagged(#[dispatch]
  Square, u32)`, was told it "cannot be built around the other fields of
  `Shape`", the enum, where the other fields are `Tagged`'s; it now names
  `Tagged`. On a dispatch struct, `self: Box<Self>` over a field that
  holds its payload plainly named the field, "`_0` holds a plain
  `Square`; hold `Box<Square>` in a `#[dispatch(deref)]` variant", where
  the field is unnamed and a struct has no variants; it now names the
  struct and the field, "`Wrapper` holds a plain `Square`; hold
  `Box<Square>` in a `#[dispatch(deref)]` field". Every sentence about the
  way a method reaches a struct's field names the struct, whose field the
  error points at. The `tests/ui` cases
  `self_return_beside_the_other_fields_of_a_variant` and
  `box_self_on_a_plain_field` are new; no other snapshot and no golden
  changed.
- Before, an associated type or const that no variant can supply in every
  build, every variant being under `#[cfg]` or reached through a field or
  a method, withheld the whole impl: its sentence was followed by rustc's
  error at every call through the trait, E0599, no method named `tag`.
  The impl is now written, with the item holding its sentence, so the
  calls resolve and the sentence stands alone. The `tests/ui` case
  `unsupplied_items_keep_the_impl` is new; no other snapshot and no golden
  changed.
- Before, a partial impl block or an inherent `#[dispatch(*)]` method over
  an enum whose every variant is under `#[cfg]` warned in the user's
  crate, unused variable, in the build that compiles every variant out,
  where no arm is left to use the arguments; the enum's own attribute did
  not, its impl being automatically derived. Such a method now allows its
  arguments to go unused, as a hand's does, where it has no body of yours
  for the rest: a body is the arm of the build that compiles every variant
  out, and an argument it leaves unused is reported as in the same impl
  written by hand. The behaviour test `enums_with_every_variant_under_cfg`
  holds the first under `#[deny(unused_variables)]` and the `tests/ui`
  case `block_body_keeps_the_users_lint` the second; no other snapshot
  and no golden changed.
- Before, a sentence about the whole of an inherent method, a return type
  that wraps `Self` where the payload's result cannot be rebuilt, told a
  method that names its variants and has a body for the rest to "give it
  a body". It now says to drop the method's `#[dispatch(..)]`, so that
  the body answers for every variant; a method without a body keeps its
  sentence. `docs/limits.md` gains the line for one such shape, a return
  that wraps `Self` twice under `with = Some` or `with = Ok`. The
  `tests/ui` case `lift_over_a_wrapped_self` is new; no other snapshot and
  no golden changed.
- The associated consts of an enum with lifetime parameters alone are
  checked to agree, as those of an enum without parameters are: no
  lifetime changes a const's value, so the check compares the payloads
  with each lifetime `'static`. Before, such an enum took the first
  variant's const without a check, so `enum E<'a> { A(P0),
  #[dispatch(deref)] B(&'a P1) }` answered `P0`'s `LEVEL` for `B` as well.
  This breaks an enum that compiled: one whose variants disagree on such a
  const, which now is an error on the variant that differs; set the const
  on the enum with `LEVEL = value`. An enum with a type or const parameter,
  and a variant reached through a field or a method, still take the first
  variant's value, and `docs/limits.md` says so. The decision record
  "Associated consts are checked unless a type can change them"
  supersedes "Associated consts are checked for four types". The
  `tests/ui` case `const_disagreement_on_a_lifetime_enum` is new; no other
  snapshot and no golden changed.
- Before, the agreement check of associated consts reported the first
  variant that disagreed and no other: the checks were one `const` block,
  whose evaluation stops at its first failure. Each check is now an item
  of its own, so every variant that disagrees is named. The `tests/ui`
  case `const_disagreement_on_two_variants` is new; the snapshots
  `associated_const_disagreement` and
  `const_disagreement_on_a_lifetime_enum` moved, rustc naming the item
  that failed `_::_` where it named `_`; the golden `associated_consts`
  shows the check as `const _: () = if ..;`.
- Before, a method that requires `Self: Sized` forwarded beside a `deref`
  variant holding a trait object was its sentence alone, in every build and
  whatever else the method met: a trait object under `#[cfg(test)]` failed
  the build that leaves it out, and the other variants' sentences, a plain
  variant under `self: Box<Self>`, were never written. The sentence is now
  the trait object's arm, under its variant's `cfg`, as every sentence
  about one variant is. The behaviour test `trait_objects_in_deref_variants`
  holds the first and the `tests/ui` case `trait_object_beside_a_plain_variant`
  the second; no other snapshot and no golden changed.
- Before, a generic associated type the enum does not declare withheld the
  impl, and its sentence was followed by rustc's error at every call
  through the trait, and the other methods' sentences were never written.
  It is now written holding its sentence, as an associated item no variant
  can supply is. The `tests/ui` case `undeclared_gat_keeps_the_impl` is
  new; no other snapshot and no golden changed.
- Before, `#[dispatch(*)]` in a partial block implementing a trait for one
  instantiation of a generic struct, `impl Area for Wrapper<Square>`, called
  the trait on the struct's parameter, E0425, cannot find type `T`, where
  the same block over a generic enum compiled. `*` now names the field as
  the header instantiates it. The behaviour test `partial_block_methods`
  holds it; no snapshot and no golden changed.
- Before, a sentence about a variant holding a parameter that a partial
  block's header instantiates, `impl Shape for Any<Square>` over `enum
  Any<T> { Plain(T), .. }`, was reported on the header's `Square`. It is
  now on the variant's payload, where the parameter is. The `tests/ui` case
  `instantiated_variant_keeps_its_sentence` is new; no other snapshot and
  no golden changed.
- `docs/partial-impl-blocks.md` says what a method a partial block writes
  or marks carries: what is written on it, with `#[inline]` where nothing
  else is, and nothing of the trait method's own attributes, which only
  the plain forwarding repeats.

## 0.7.3 - 2026-11-13

The macro crate moves to `syn` 3, so a build that has left `syn` 2 gains
no second copy of it, and the README becomes the case for the crate, with
its manual in pages under `docs/`. No expansion and no sentence changed.

- The macro crate parses with `syn` 3, where it used `syn` 2. coreutils,
  purecrypto, toasty and lofty build `syn` 3 only, as `serde_derive` and
  `thiserror` 2 do, and `whichever` would have put `syn` 2 back into those
  builds; coreutils' `deny.toml` denies the duplicate. No expansion and no
  sentence changed: the 32 expansion goldens and the 120 `tests/ui`
  snapshots are the ones 0.7.2 had. The minimum supported Rust version
  stays 1.88. A tree still wholly on `syn` 2 gains `syn` 3 instead; the
  decision record weighs the two.
- The README is the case for the crate and no longer its manual: what the
  attributes write, the open set, a `Box<dyn Trait>` variant beside the
  inline ones, where `enum_dispatch` and the crates after it stop, a few
  lines each, and the measurements. The options, routes, inherent
  methods, restatements, partial impl blocks, struct forwarding, the table
  of what each trait item becomes, the limits and the compatibility notes
  moved, wording intact, to a page each under `docs/`, with the full
  comparison table in `docs/comparison.md`; `docs/README.md` says which to
  read when. `examples/storage.rs` is the trait `docs/comparison.md` shows
  that `enum_dispatch` cannot take, an associated type unified through
  `Into`, a `&Self` argument, an `Option<Self>` return and `Display`, with
  each point asserted. No code changed.

## 0.7.2 - 2026-11-12

What the search of 2026-11-12 over about 200 crates found. darling_core
0.24.1's error iterator is an enum over `iter::Once` and `vec::IntoIter`
that implements `Iterator` with `next` alone, so its `size_hint` is
`(0, None)` and the enum is neither double-ended nor exact-size although
both payloads are. sonic-rs 0.5.10's `IterInner` implements
`ExactSizeIterator::len` exactly while its `size_hint` reports `(0, None)`.
rodio 0.23.0's `Source` has 7 required methods and about 29 provided
combinators such as `fn buffered(self) -> Buffered<Self>`, and its
`DecoderImpl` forwards the required ones by hand over 5 `#[cfg]` variants.
poem 3.1.12's `CombinedStream`, pgwire 0.42.2's `MaybeTls`,
tokio-postgres 0.7.18's socket and `MaybeTlsStream` and suppaftp 12.2.1's
`DataStream` implement tokio's `AsyncWrite` with the three required
methods alone, so each reports `is_write_vectored` as `false` over streams
that answer `true`. hyper 1.12.0, which serves poem's, then copies every
buffer it writes. toml_edit 0.25's `Value` forwards `decor`, `span` and
`despan` by hand over 7 variants, and cannot take `whichever` with its
default feature, which reaches back to toml_edit through
`proc-macro-crate`.

Three fixes follow the additions, from the surveys of 2026-11-12 of the
four projects that search ranked highest: two from martin-core 0.17.0 and
one from pingora-core 0.9.1.

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
- The README's "Traits from `std` and other crates" gains a recipe for
  tokio's `AsyncWrite` over an enum of `Unpin` streams, and the crate docs
  a warning: a restatement forwards only the methods it lists, so one that
  restates the required methods alone leaves the enum on the trait's
  defaults, and `poll_write_vectored` and `is_write_vectored` belong in it.
  Nothing changed in the macro. tokio is not a dependency, so the shape is
  held by a behaviour test with a local stand-in for tokio's trait, with
  and without the provided methods.
- The README's "Compatibility" says that a crate in `proc-macro-crate`'s
  own dependency tree takes `whichever` with `default-features = false`,
  since with the default feature it would depend on itself, a cycle Cargo
  rejects. toml_edit 0.25's `Value` raised it. No code changed.
- Before, a method a partial impl block marks with `#[dispatch(..)]` under
  `#[cfg]` did not follow the `cfg`, and silently, with a wrong answer and
  no error. `#[cfg(feature = "x")] #[dispatch(default)] fn name(&self) ->
  &'static str;` left `name` to the trait's default in every build, so
  with the feature off a payload's override was never called; a forwarded
  mark under `#[cfg]` wrote nothing for its method where the `cfg` did not
  hold, so a provided method took the trait's default there; and of two
  marks of one name under opposite `cfg`s the second replaced the first,
  so where only the first one's held nothing was written. A mark under
  `#[cfg]` now holds where the `cfg` does, as a method the block writes
  does, and the other builds go to a mark of the same name without a
  `#[cfg]`, or else to every variant. A required method marked
  `#[dispatch(default)]` under a `cfg` keeps its sentence where the `cfg`
  holds and is forwarded where it does not.
- Before, an inherent method whose arm is a place, `field = name` or
  `value`, handed the place to `with = Some` by value, so `Some(inner.name)`
  on `-> Option<&String>` was rustc's E0308, expected `&String`, found
  `String`, and `via = *, value, with = Some` on `-> Option<&dyn Trait>`
  failed the same way. Under `with = Some` or `with = Ok` the borrow is now
  read from the return type's first type argument: `-> Option<&T>` gives
  `Some(&inner.name)`, `-> Option<&mut T>` on `&mut self` gives
  `Some(&mut inner.name)` and `-> Result<&T, E>` gives `Ok(&inner.name)`,
  and a `String` field on `-> Option<&str>` compiles too. Any other lift,
  a function or `Option::from` or `Into::into`, is handed the place by
  value as before, so a field that is itself a reference still passes
  through as it is. Nothing that compiled before stops compiling.
- Before, every `&self` and `&mut self` method of an enum whose variants
  are all under `#[cfg]` was rustc's E0004, `type &AnyShape is non-empty`,
  in a build that compiles none of them, as martin-core's `BackendSource`
  does under its `_tiles` feature alone: a reference always counts as
  inhabited, so `match self {}` is not exhaustive. Over such an enum a
  reference receiver's `match` now looks at the enum itself and the arms
  borrow the payload, `match *self { AnyShape::Circle(ref __inner) => ..
  }`, with `ref mut` for an exclusive receiver and `*Pin::get_mut(self)`
  for a pinned one, which is what a hand writes and is exhaustive with no
  arm left. An enum with a variant that is always compiled in expands
  exactly as before, and so do by-value and `Box<Self>` receivers and a
  method with a `Self` argument. The decision record says why the two
  spellings stand side by side.

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
