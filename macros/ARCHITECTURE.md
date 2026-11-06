# Architecture of `whichever-macros`

Status: landed. Written 2026-11-02 as a plan against 0.4.0 and carried out
the same day, stage by stage; a second round followed from two independent
reviews of the result. Sections 1 and 3 describe the crate as it is,
section 2 records what the shape was before, and each stage in section 4
carries the status line written when it landed.

The words are the README's and those of [`docs/vision.md`](../docs/vision.md):
carrier macro, implementation macro, forwarding impl, arm, rewrap, mismatch,
agreement, override, conversions, restatement, partial impl block, inherent
dispatch, fallback body, receiver, shape, limit. The one internal word this
file adds is *subject*, the enum or struct an impl is written for.

## Contents

1. How an expansion flows
2. What is wrong with the current shape
3. Target design
4. Plan
5. Invariants every stage keeps
6. Open decisions

---

## 1. How an expansion flows

Nothing is kept between expansions, so every piece of information travels
as tokens through `macro_rules!` carriers. Four proc macros take part.

| Step | Where | What happens |
| --- | --- | --- |
| `#[dispatchable]` on a trait | the trait's crate | The trait is re-emitted. A carrier macro holding the trait's signatures, with the attributes the generated items carry and the names of the methods that have a default, is emitted next to it and re-exported under the trait's own name, in the macro namespace; a trait with items under `#[cfg]` gets one definition of the carrier per combination of its distinct predicates, each under that combination's `cfg` and holding the items that exist under it with their `cfg` attributes taken off, so that the predicate is decided in this crate. `crate::` paths in the tokens become `$crate::`. With `remote = path`, only the carrier is emitted. |
| `#[dispatch(Trait, ..)]` on an enum or struct | the subject's crate | The subject is re-emitted without its `#[dispatch]` attributes, unless it restates another crate's enum with `remote = path`, in which case nothing is emitted for it. The conversions are written. A crate-local carrier holding the subject's tokens is emitted and re-exported under the subject's name. For every trait named, `Trait! { @v1 facade enum { subject } spec { options } [remote { path }] }` is emitted. |
| The trait's carrier | expands in the subject's crate | Prepends `trait { tokens } [remote { path }]` and calls `::facade::__private::implement!`. |
| `implement!` | | Parses the trait, the subject, the options and an optional partial impl block; writes the forwarding impl, the agreement checks and the shape errors. |
| `#[dispatch]` on an impl block | the subject's crate | Emits `Subject! { @v1 facade [partial { options }] impl { block } }`. |
| The subject's carrier | | Prepends `[rejected] enum { subject tokens }` and calls `implement_block!`. |
| `implement_block!` | | An inherent block: rewrites every marked method into a forwarding method with the body as the fallback. A trait impl block: re-emits `Trait! { .. impl { block } }`, so the trait's carrier and `implement!` finish the job. |

Inside `implement!`, the subject yields entries: a target per forwardable
variant or the one field of a struct, and an unforwardable entry per variant
of another shape, kept so that errors carry its `#[cfg]`. Each trait method
is classified by receiver, by arguments and by return shape, the trait's
generics are substituted with the arguments the subject named, and one
`match` is written with a fully qualified call per arm. Associated types and
consts are taken from the first target and checked against the rest in a
`const _` block; the type check's helpers are associated functions of a
wrapper struct of the subject declared in that block, so that the subject's
parameters belong to their impl and the bounds its fields imply hold there.

## 2. What was wrong with the shape of 0.4.0

Written before the work, in the present tense of 0.4.0; section 4 says
what became of each point.

- **One file.** `src/lib.rs` is 2,789 lines; the five largest functions are
  205, 194, 140, 137 and 107 lines long. The section comments already mark
  the module seams. Nothing inside is unit-tested; every test goes through
  rustc.
- **Two forwarding generators.** `expand_method` (trait methods) and
  `forward_method` (inherent methods) each build the same `match` and
  diverge in policy: trait methods get parameters renamed to `__argument0`,
  `&Self` arguments checked, a fully qualified callee and an explicit deref;
  inherent methods keep their parameter names, skip the `&Self` check, use
  method-call syntax and rely on auto-deref. The trait path stops at the
  first method it cannot forward; the inherent path collects every error.
- **A flat return model.** `ReturnShape` has five values for a
  two-dimensional question, what wraps the result and what the inner value
  needs. `Result<Self, Self::Error>` with `Error = X` overridden takes the
  rewrap branch and the error is never converted; rustc reports a type
  mismatch at the trait's `#[dispatchable]` line.
- **A half-finished subject.** `Subject::is_enum()` leaks into the method
  generator and the conversions. `Target` carries `from` and `try_from`,
  which only a variant has. A struct's field never has cfgs. A struct's
  carrier is named `__whichever_enum_..`.
- **An unstated protocol.** The two `@v1` message grammars are encoded at
  four `quote!` sites and decoded by two hand-written parsers that detect
  optional segments with `peek(Ident)`. `remote { }` was added in 0.3.0
  without touching the version. `implement_inherent!` also routes partial
  trait impls.
- **Three option grammars.** The subject attribute and inherent forwarding
  have hand-written `Parse` impls, variants use `parse_nested_meta`, and
  `#[dispatchable]` has a third parser. Subject-level `from` and `try_from`
  ride on trait attributes and travel through the carrier only to be
  ignored. `Name = ..` is parsed before the trait is known, so type versus
  const is decided by a speculative parse.
- **Carrier names rest on two accidents.** `DefaultHasher` is documented as
  unstable across releases; the scheme works because one build uses one
  compiler. Hashing token positions is what sets the crate's minimum Rust
  version, since `Span::file` and `Span::line` landed in 1.88.
- **Renamed parameters are visible.** `fn write(&mut self, __argument0:
  &[u8])` shows in expansions and in anything that reads the impl's
  signature, and a `mismatch` expression cannot name the arguments.

## 3. Target design

### 3.1 Module map

```
src/
  lib.rs           crate docs; the four proc-macro entry points; `mod` lines
  dispatchable.rs  `#[dispatchable]`: the trait's carrier, its method attributes checked
  dispatch.rs      `#[dispatch]` on a subject or an impl block: what is emitted
  naming.rs        how the carrier macros are named: a fixed hash of text and token positions
  subject.rs       Subject, Target, Place, Entry; reading variants and fields
  route.rs         the route: how an arm reaches its target from the payload
  options.rs       every `#[dispatch(..)]` and `#[dispatchable(..)]` grammar, one struct per position
  protocol.rs      the `@v1` messages: one struct each, `ToTokens` and `Parse`, and the carrier builder
  implement.rs     `implement!`: the forwarding impl for one trait
  block.rs         `implement_block!`: inherent blocks and partial impl blocks
  forwarding.rs    the engine: a plan per method, arms per target, one renderer
  shape.rs         classifying receivers, arguments, return types and generics
  substitution.rs  the trait's generics replaced by the subject's arguments
  agreement.rs     the associated type and const checks
  conversions.rs   `From` and `TryFrom`
```

Stage 1 created the files by moving code; later stages changed what is in
them. `shape.rs`, `options.rs` and `protocol.rs` have unit tests through
`parse_quote!`, which the monolith made impractical; `substitution.rs` can
follow the same pattern.

### 3.2 The forwarding engine

One method body is one value:

```rust
struct MethodPlan {              // read off the signature once
    receiver: ReceiverKind,      // Value(access) | Boxed | Pinned(access)
    self_token: Token![self],    // reused, so the `match` shares its hygiene
    arguments: Vec<Ident>,       // the bindings after the receiver
    same_variant: Vec<(usize, Access)>, // the `Self` arguments, matched beside the receiver
    turbofish: Option<TokenStream>,     // the generics a call spells out when inference could not
    return_model: ReturnModel,
    future: bool, awaited: bool, unsafety: bool,
}

struct ArmContext<'a> {          // what every arm of one method shares
    callee: Callee<'a>,          // Qualified { trait_path, method } | Method { name }
    method: &'a Ident, subject: &'a Ident,
    prefix: Path,                // the subject's name, or `Self`
    lift: Option<&'a Path>,      // the inherent `with = path`
    site: Span,                  // the span every written token carries
}

struct Arm { cfgs: Vec<Attribute>, pattern: TokenStream, value: TokenStream }

struct Forwarding {
    scrutinee: TokenStream,      // `self`, `*self`, `Pin::get_mut(self)`, or a tuple with the Self arguments
    arms: Vec<Arm>,              // one per selected target, plus the unforwardable ones
    fallback: Option<Arm>,       // the mismatch arm, or the inherent fallback body
    future: bool,                // wrap in `async move { .. }`
}
```

Both entry points do the same three things: read the signature into a
`MethodPlan`, build an `Arm` per selected target with `MethodPlan::arm`
and the shared `ArmContext`, and hand the `Forwarding` to the one renderer.
What differs between a trait method and an inherent method becomes data:
the `Callee`, which also tells the two paths apart and so supplies the way
out each error message offers, which targets are selected, the fallback,
the `with = path` lift, and two policies that are deliberate once named.
`MethodPlan::fallback` writes the arm for two different variants and binds
again the same-variant arguments the scrutinee moved, so that a `mismatch`
on the trait method may use them. Every token the engine writes carries the
context's `site`, the subject's or the method's own span: a proc macro's
call site inside a carrier's expansion is the trait's attribute, which may
be in another crate. In a trait method
`Self` is the implementor, so a `&Self` argument is matched on the same
variant and needs a mismatch; in an inherent method `Self` is the enum
itself, so a `&Self` argument is handed to the payload's method as it is,
and a payload method taking the enum is a legitimate target. A trait
method's parameter names come from a trait the enum's crate may not own,
and are generated; an inherent method's are the user's, and are kept. The
one difference that is not deliberate, the trait path renaming parameters
that could keep their names, goes in stage 6.

### 3.3 The return model

```rust
enum Conversion {                 // `future` travels beside it in the plan
    Keep,                         // the payload's result as it is
    Rewrap,                       // `Self`: the variant's constructor around it
    Into,                         // `Self::X` with `X` overridden: `Into::into`
    Map(Box<Conversion>),         // `Option<T>`, `io::Result<T>`: `.map(path)`
    MapErr(Box<Conversion>, Box<Conversion>),  // `Result<T, E>`: `.map(path).map_err(path)`
    Tuple(Vec<Conversion>),       // `(A, B)`: taken apart, converted, built again
}
```

Since 2026-11-05 the model is a tree over the return type, where it was a
wrapper around a value. Rendering is one recursive function: a rewrap is
the constructor and an `Into` is `Into::into`, each applied directly or as
the path a `.map(..)` or a `.map_err(..)` takes, and a tuple is `{ let (a,
b) = call; (Variant(a), b) }`. The rule that keeps rule 1 is the
classifier's: a conversion under a wrapper has to be a path, since
`.map(|x| ..)` is a closure the vision forbids, so a tuple under an
`Option` is a limit with the one sentence, while a tuple holding a
wrapper is fine. The recognition rules stay syntactic: `Option` and
`Result` by the name a path ends in, `Self::X` as a two-segment path,
`impl Future` by its `Output` binding, and any other `Self` in a return
type is a limit. `Result<Self, Self::Error>` with `Error` overridden
rewraps and converts (golden `overridden_results`); `(Self, u32)` and
`(Self::Rest, Option<Self>)` take the tuple apart (golden
`tuple_returns`).

### 3.4 The subject

`Subject` keeps its two cases and answers the questions the generators ask
instead of being asked what it is: the entries, the pattern prefix, whether
two values of it can hold different targets (an enum can, so a `Self`
argument needs a mismatch arm), whether it has conversions, and the shape
errors of its unforwardable entries. `from` and `try_from` move from
`Target` into `Place::Variant`, since only a variant has them. The subject's
carrier is named `__whichever_dispatch_..` for both cases, after the
attribute that writes it. Since 2026-11-05 a `Target` carries a `Route`,
the steps from the bound payload to the value the trait is called on,
where it carried a `deref` flag: `deref` is the route `*`, a field or a
method is a step, and `Place::Variant` holds the marked field of a variant
with several. `Target::target_type` is `None` where the route leaves the
type unnameable, and the engine then qualifies the call by the trait
alone, `<_ as Trait>::method`. The inherent path's `Callee::Place` is the
arm that calls nothing: the target itself, or a field of it.

### 3.5 The protocol

`protocol.rs` holds one struct per message, each with `ToTokens` for the
sender and `Parse` for the receiver, the one builder both carrier macros
are written by, and the grammar in one place:

```
@v1 <facade> enum { <subject> } spec { <options> } [impl { <block> }]       sent by #[dispatch] to a trait's carrier
@v1 trait { <trait> } [remote { <path> }] [provided { <method>, .. }] <the rest of the message above>   what the carrier hands implement!
@v1 <facade> [partial { <options> }] impl { <block> }                        sent by #[dispatch] to a subject's carrier
@v1 [rejected] facade { <facade> } [traits { <paths> }] enum { <subject> } <the rest above>   what the carrier hands implement_block!
```

The options travel typed: a `TraitRequest` carries a `TraitSpec`, a
`BlockRequest` the partial block's options, parsed and checked at the
attribute. The trait's carrier is compiled into the trait's crate and read
by whatever version of this crate the subject's crate builds with, so the
first two lines are a compatibility surface. Between two keywords a message
is a run of segments, `tag` or `tag { .. }`; a segment is only ever added,
and the readers skip a tag they do not know, so an older reader accepts a
newer carrier, which a unit test and a behaviour test with a carrier of the
0.4.0 shape pin. `v1` becomes `v2` when a segment changes meaning or order.
The subject's carrier never leaves its crate and forwards the facade's
name, so a partial impl needs no second lookup, and the traits the subject's
own attributes dispatch, so a partial block of one of them is reported as
the second impl it would be. The facade pins the macros
crate exactly: `__private` and the generated code are one interface.
`implement_inherent!` is renamed `implement_block!`; the name `implement!`
cannot change.

### 3.6 Options

`options.rs` owns every `#[dispatch(..)]` and `#[dispatchable(..)]` grammar,
one typed struct per position. On a subject the attribute is a
`DispatchAttribute`: a `TraitSpec`, the trait's path and options, which is
what travels to `implement!` and what a partial block's attribute holds,
beside the `Switches`, the subject's `from` and `try_from`, each with the
span it was written at, read where the subject is expanded and never sent.
The right-hand side of `Name = ..` stays a `TokenStream` until `implement!`
knows whether `Name` is an associated type or const, and is parsed as a
`Type` or an `Expr` then. The grammar rejects an option or a variant given
twice; `implement!` rejects an override or a `mismatch` naming nothing in
the trait; `#[dispatchable]` rejects a method attribute that is not one
`mismatch = ..` on a method with a `Self` argument.

### 3.7 Carrier naming

The hash is FNV-1a over the trait's text and token positions, written out
in `naming.rs`, so the name is stable by construction rather than by the
accident of one compiler per build. Token positions stay in the name, and
with them the 1.88 minimum.

### 3.8 Diagnostics policy

Every generator collects errors with `syn::Error::combine` and reports all
of them, and every generator emits the item. A trait method that cannot be
forwarded is written with its signature and its sentence as its body, a
`compile_error!` rustc treats as an error expression and type-checks no
further, so the impl is complete, the calls around it resolve, and the
sentence stands alone; only a signature the substitution could not write,
a projection it could not qualify, withholds the impl, as does an
associated item that cannot be supplied, since neither has a body to hold
an error. The inherent path keeps the block and the methods it could
forward, for the same reason. At the subject the unit of rejection is the
attribute: the
subject and its carrier are always emitted, each attribute that parses is
expanded, the conversions that can be written are written, and the errors
follow; "rejected", the flag the carrier hands the subject's impl blocks, is
reserved for a subject whose variants or field cannot be read, since then
the blocks could only repeat that error. Each trait's carrier invocation
carries the span of its own attribute, so an error rustc finds in one
trait's impl is labelled on that attribute and not on the first of the
stack. A trait method's parameter keeps its own name when its
pattern is a plain identifier and gets `__argument<n>` otherwise, so
expansions read like the trait, and a `mismatch` expression written on the
trait method or on the enum may name the arguments, a moved `Self` argument
included, which the fallback arm binds again, and a `&self` receiver. The
two live in different hygiene contexts, the trait's tokens having come
through a `macro_rules!` body, so the engine resolves the receiver's `self`
token and the written argument bindings at the chosen expression's span
with `Span::resolved_at` (stable since 1.45); the arms use that same `self`
token. A rustc error inside generated code is accepted only where a
hand-written impl gets the same error on the same user token, a variant's
payload or a trait method; one on an attribute is a defect. So no module
that writes generated code uses a bare `quote!` or `parse_quote!`: every
token carries the subject's, the method's or the payload's span, and the
unit test in `spans.rs` reads those modules and names a token written at
the call site, which inside `implement!` is the trait's attribute.

## 4. Plan

Each stage is one commit or a few, with the suite green at every commit:
`cargo test --workspace`, `cargo test --test expand -- --ignored`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`,
`cargo doc` with warnings denied, and the compile-fail cases on stable.

### Stage 1: split `lib.rs` into the module map

Status: landed. Code moves only; every golden and `.stderr` file is
unchanged. The four proc-macro entry points stay in `lib.rs`, where Rust
requires them.

Code moves; nothing changes. Done when the expansion goldens are unchanged
byte for byte, the `.stderr` files are unchanged, and no function is longer
than it was.

### Stage 2: one forwarding engine

Status: landed. `forwarding.rs` holds `MethodPlan`, `Arms`, `Callee`,
`Arm` and `Forwarding`; `expand_method` lives next to the trait impl in
`implement.rs` and `forward_method` in `inherent.rs`, both as plans fed to
the engine. Every golden and `.stderr` file is unchanged.

`forwarding.rs` with `Forwarding`, `Arm`, `Call` and the renderer;
`expand_method` and `forward_method` become plans fed to it. Done when both
paths go through the engine and the goldens are unchanged. Two diagnostics
the trait path had and the inherent path lacked come along for free: a
`deref` target in a method returning `Self`, and an `unsafe` method's calls
wrapped in `unsafe { }`.

### Stage 3: the return model

Status: landed. `Result<Self, Self::Error>` with `Error` overridden now
rewraps and converts (golden `overridden_results`, behaviour test
`overridden_results`), and so do `Option<Self::X>` and `Result<Self::X, E>`
with `X` overridden, which were rustc type errors before. A `Self` in a
`Result`'s error position is reported as a limit instead of reaching rustc.
Every other golden and `.stderr` file is unchanged.

`ReturnShape` is replaced by `ReturnModel`. Done when `Result<Self,
Self::Error>` with an override compiles and has a behaviour test and a
golden, and every other golden is unchanged.

### Stage 4: the subject and the protocol

Status: landed. `Place::Variant` holds the conversion switches, `Subject`
answers `can_mismatch`, `has_conversions` and `target_noun` instead of
`is_enum`, `protocol.rs` holds the four messages and the one carrier
builder, `implement_inherent!` is `implement_block!`, and the subject's
carrier is named `__whichever_dispatch_..`. Every golden changed on that
one alias line and nowhere else; every `.stderr` file is unchanged.

`Place::Variant` takes the conversion switches, `is_enum()` leaves the
generators, `protocol.rs` gets the four messages with encoder and decoder
side by side, `implement_inherent!` becomes `implement_block!`, the
subject's carrier gets its name. Done when no `quote!` outside `protocol.rs`
spells a message and the goldens are unchanged apart from the carrier's
name.

### Stage 5: one option grammar

Status: landed. `options.rs` holds every grammar; `Override` keeps the
tokens written and reads them as a type or a value once the trait says
which, so a const override written as a tuple of paths works (behaviour
test `tuple_const_override`); the subject's switches are read at the
subject and no longer travel through the carrier; the `std` path check and
the switch checks are methods of `TraitSpec`. A trait path on a partial
block's attribute is now reported instead of being sent on as a stray
option. Every golden and `.stderr` file is unchanged.

`options.rs` as in 3.6; `Override` parses lazily. Done when the three
parsers are one module, the `.stderr` files are unchanged or better, and a
tuple-valued const override is accepted.

### Stage 6: naming, parameter names, error collection

Status: landed, in three commits. FNV-1a replaces `DefaultHasher`, so every
carrier name changed once (goldens: the alias lines only). Trait methods
keep their written parameter names (goldens: `__argument0` became `buf`,
`f`, `cx`, `other`, `factor`, `value`, `max`). `implement!` reports every
failing item (compile-fail case `two_unsupported_methods`); no other
`.stderr` file changed.

FNV-1a replaces `DefaultHasher`; parameters keep their names; both
generators collect errors. Done when the goldens show the trait's parameter
names, a compile-fail case shows two bad methods reported together, and the
cross-crate smoke crates still build.

### Stage 7: record

Status: landed. The changelog's unreleased entry, the crate docs and the
README on what an override converts, this file's status lines, unit tests
for the return model, and a run on 1.88 as well as stable. A terminology
file drafted beside this work, whose entry for the implementation macros
still named `implement_inherent!`, was folded into the README and
`docs/vision.md` before it landed.

### Round two

After stage 7 two reviews read the result fresh, one the design and one
the behaviour through probe crates; their findings, consolidated, landed in
five more commits, each through the same gate.

### Stage 8: structure

Status: landed. The protocol's option fields are typed, its readers skip
unknown segments, and the subject's carrier forwards the facade; carrier
names live in `naming.rs`; `inherent.rs` is `block.rs`; `Arms` is
`ArmContext`, with the hints derived from the `Callee`; the rendering
helpers left `shape.rs`; `ImplHeader` names the impl's header; three
duplications went; the facade pins the macros crate exactly. Two
compile-fail snapshots narrowed to the switch they report.

### Stage 9: validation

Status: landed. Eight shapes that passed silently or reached rustc badly
are one sentence each, a rejected inherent block keeps its methods, and the
`Box<Self>` message spells the payload as written. Nine compile-fail cases.

### Stage 10: generated code

Status: landed. `unsafe trait` gives `unsafe impl`; a method's own
`#[inline(..)]` is carried; a generic method is called with its parameters
spelled out when no argument fixes them; the associated items come from the
first variant without a `#[cfg]`, with an error when there is none, and an
associated item under `#[cfg]` stays under it; the fallback arm rebinds
moved `Self` arguments; the impl header and every engine token carry the
subject's or the method's span. Two goldens, three behaviour tests, one
compile-fail case; two existing snapshots moved from the trait's attribute
to the enum or the method.

### Stage 11: tests

Status: landed. Kept parameter names under every name the engine uses, a
carrier of the 0.4.0 shape read by the new reader, an inherent block on a
struct, a partial block over a `foreign` declaration, the enum-side
`mismatch` hygiene boundary (lifted since, 3.8), and unit tests for `protocol.rs`,
`options.rs` and `shape.rs`.

### Stage 12: record

Status: landed. The README's `foreign::Error` example, which did not
compile, the semantics tables, the `mismatch` hygiene note, the changelog
and this file.

### Round three

Nine bugs filed from the ambassador and enum_delegate surveys, reproduced
on 0.5.0, and the gap items with a settled design, landed on 2026-11-02 in
one round through the same gate.

### Stage 13: the surveys' bugs

Status: landed. The unit of rejection at a subject is the attribute (3.8);
the subject's carrier names the traits the subject dispatches, so a partial
block of one is reported as the second impl; a struct forwards opaque
returns as they are, and an opaque type nested in an enum's return type is
found; `classify_return` takes a `ReturnSite`, the subject's name and
whether it has one target, so the `Self` limit names the subject and the
way out of the method's own path; an inherent method of a struct unwraps
its `Self` arguments as trait dispatch does; a hand-written `#[allow]` on a
trait item is carried onto the generated item; a partial block's header may
rename the subject's parameters, and `Substitution` serves the subject's
generics as well as the trait's, reporting a projection it cannot qualify;
a generic associated type the block declares is accepted and one it does
not is pointed at the block; an override naming nothing in the trait says
what was probably meant; a variant's shape error names what the variant
holds and the inherent way around it, and an enum whose every variant is
rejected gets no impl on top; `#[dispatchable]` reports `#[async_trait]`
below it and `#[typetag::serde]` above it; a second `#[dispatch]` on an
inherent method, a closure after `with`, and `Name<..> = ..` for a generic
associated type are one sentence each. Goldens: `carried_attributes` (the
`#[allow]` lines), `opaque_through_a_struct` and `gat_in_a_partial_block`
(new). Compile-fail snapshots: the attribute-stack label moved from "in this
attribute macro expansion" to "in this macro invocation" on the trait path
in eleven cases; the variant shape sentences in `struct_variant`,
`two_field_variant`, `unit_variant`, `cfg_enabled_bad_variant`,
`rejected_variant_keeps_enum` and `inherent_all_with_unforwardable`, the two
of which lost their "needs at least one variant" follow-up;
`generic_associated_type`, `generic_trait_without_arguments`,
`unknown_option_names` and `self_in_aliased_return` reworded; fourteen new
cases.

### Stage 15: the review of 2026-11-05

Status: landed on 2026-11-06, one commit per item, on the branch
`execute-review-2026-11-05`. The fallback arm and its lint
allowance only when a mismatch can happen (golden
`self_argument_and_return`); the bound a hand writes for a type-parameter
payload (golden `generic_payloads`); the enum-side `mismatch` resolved at
its own site (`enum_mismatch_names_argument` became a behaviour test); a
rejected trait method written with its error as its body
(`rejected_method_keeps_the_impl`); a trait item's `cfg` decided in the
trait's crate by one carrier per combination (`cfg_on_trait_items`,
`too_many_cfg_predicates`, the smoke crates' `Gated`); the carrier
holding signatures only, with the `provided` segment; the impl header
documented (`impl_header`); routes (`routes`, four compile-fail cases,
seven snapshots reworded for the new options and the marker); the partial
block's method attributes, with the target selection shared between the
inherent path and the trait path (`partial_block_methods`, three
compile-fail cases); the by-reference `TryFrom` forms and `into = Target`
(`conversions`, one compile-fail case); a struct's field naming its traits,
with `Subject::entries_for` choosing the target per trait
(`per_trait_fields`, two compile-fail cases); the restated subject, whose
real path travels in a `remote` segment of the request and of the subject's
carrier and is the impl's self type and the patterns' prefix
(`restated_enum`, two compile-fail cases, the smoke crates' `Either`);
the return model as a tree with tuple returns (`tuple_returns`, one
compile-fail case, `struct_self_return_limit` re-pinned on `Vec<Self>`);
every sentence pinned by a snapshot (`limits.rs`, thirty cases);
`proc-macro-crate` behind the `renamed-dependency` feature and a
semver-checks job; the carrier names under unit test; the quoted numbers
checked in CI; the behaviour tests split by capability.

### Stage 14: the enum_dispatch page-1 surveys' bugs

Status: landed, after 0.5.1. The associated-type agreement check's helpers
are associated functions of a local wrapper struct of the subject, so a
lifetime the first payload does not mention is never passed (E0794) and the
bounds the payloads imply hold (E0309); a partial block's method under
`#[cfg]` is forwarded under the complement of its predicates, and an
associated type or const under one is a sentence. Goldens: the check block
changed shape in `carried_attributes` and `recursive`;
`conditional_partial_method` is new. Compile-fail snapshots:
`associated_type_disagreement` moved with the helper's new home; one new
case.

### Stage 16: the review of 2026-11-06

Status: in progress on the branch `execute-review-2026-11-06`, one commit
per item in the order of `notes/review-2026-11-06.md` section 8. Every
token the engine writes carries a user span through `at`, located at the
subject's, the method's or the payload's token and resolved at the
expansion's own site, and `spans.rs` holds it (snapshots:
`associated_const_disagreement`, `associated_type_disagreement`,
`gat_payload_without_into`, `gat_undeclared_in_block`, `missing_impl`,
`partial_without_receiverless` and `private_inherent_method_span` moved
their labels from the trait's attribute to the enum, the block or the
method; no golden changed). The agreement block asks the first variant for
the impl (goldens `carried_attributes` and `recursive` gained the
`__first_implements` line; `missing_impl_with_associated_type` is new).
A deprecated trait item or variant is forwarded under `#[allow(deprecated)]`
(golden `deprecated` and the behaviour test `deprecated_items` are new;
the carrier keeps `#[deprecated]` beside `#[inline]`).
A `Self: Sized` method over a trait object payload is one sentence
(`dyn_payload_under_sized_method` and the behaviour test
`trait_objects_in_deref_variants` are new; `#[dispatch(default)]` over a
restatement needs no `provided` name).

## 5. Invariants every stage keeps

- The generated impl is the one a careful hand writes: a `match`, one fully
  qualified call per arm, `#[inline]`, no conversion unless an option asks
  for one. `benchmarks/scripts/codegen.py` keeps checking this.
- Generated bindings use `Span::mixed_site()`; the receiver uses the trait
  method's own `self` token; a variant's `#[cfg]` is carried onto its arm,
  a trait item's is decided by which carrier the trait's crate compiled.
- Every token the engine writes carries a user span, never the call site:
  `spans.rs` reads the engine's modules and names a bare `quote!`.
- Nothing is kept between expansions.
- `implement!` keeps its name and accepts every `@v1` message a 0.x carrier
  emits; the facade and the macros crate move as one version.
- A rejected subject is still emitted, with its carrier, so the code around
  it compiles.
- The facade stays `no_std` and dependency-free; the minimum Rust version
  stays 1.88.
- Every change to a golden or a `.stderr` file is listed in the stage's
  status line.
- Every sentence the macros write has a `tests/ui` snapshot: `limits.rs`
  reads the sources and the snapshots and names the ones that do not.

## 6. Open decisions

- *Subject* is the internal name for the enum or struct and the benchmark
  word for a strategy under test. Keeping it is fine while it stays out of
  user-facing text; renaming it is a one-module change.
- Subject-level `from` and `try_from` on a trait attribute are accepted
  today. Stage 5 keeps accepting them; rejecting them is a breaking change
  for a later release.
- Settled on 2026-11-05: `proc-macro-crate` sits behind the default
  feature `renamed-dependency`; without it the facade is `whichever` and
  the macro crate's tree is `proc-macro2`, `quote` and `syn`.
- An enum with two partial impl blocks, or a dispatched trait and a partial
  block, reports an unforwardable variant once per block. Nothing stateless
  can tell the blocks apart; the duplicate is left.
- Under `*`, an unforwardable variant and a body give two errors in turn,
  "the body would never run" and then the arm's. Reading `*` as "every
  forwardable variant" with the body for the rest would be one; a design
  call for later.
- Settled on 2026-11-05: the trait path emits the impl with a failing
  method's sentence as that method's body (3.8). Withholding the impl had
  traded a cascade at every call site for a tidy compile-fail output;
  emitting the method with its error as its body keeps both.
