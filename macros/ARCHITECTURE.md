# Architecture of `whichever-macros`

Status: landed. Written 2026-11-02 as a plan against 0.4.0 and carried out
the same day, stage by stage. Sections 1 and 3 describe the crate as it is,
section 2 records what the shape was before, and each stage in section 4
carries the status line written when it landed.

The words are those of [`docs/terminology.md`](../docs/terminology.md):
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
| `#[dispatchable]` on a trait | the trait's crate | The trait is re-emitted. A carrier macro holding the trait's tokens is emitted next to it and re-exported under the trait's own name, in the macro namespace. `crate::` paths in the tokens become `$crate::`. With `remote = path`, only the carrier is emitted. |
| `#[dispatch(Trait, ..)]` on an enum or struct | the subject's crate | The subject is re-emitted without its `#[dispatch]` attributes. The conversions are written. A crate-local carrier holding the subject's tokens is emitted and re-exported under the subject's name. For every trait named, `Trait! { @v1 facade enum { subject } spec { options } }` is emitted. |
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
`const _` block.

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
  dispatchable.rs  `#[dispatchable]`: options, carrier emission, naming
  dispatch.rs      `#[dispatch]` on a subject or an impl block: what is emitted
  subject.rs       Subject, Target, Place, Entry; reading variants and fields
  options.rs       every `#[dispatch(..)]` grammar, one parser per position
  protocol.rs      the `@v1` messages: one struct each, `ToTokens` and `Parse`
  implement.rs     `implement!`: the forwarding impl for one trait
  inherent.rs      `implement_block!`: inherent blocks and partial impl blocks
  forwarding.rs    the engine: a plan per method, arms per target, one renderer
  shape.rs         classifying receivers, arguments and return types
  substitution.rs  the trait's generics replaced by the subject's arguments
  agreement.rs     the associated type and const checks
  conversions.rs   `From` and `TryFrom`
```

Stage 1 creates the files by moving code; later stages change what is in
them. `shape.rs` gets unit tests through `parse_quote!`, which the monolith
made impractical; `substitution.rs` and `options.rs` can follow the same
pattern.

### 3.2 The forwarding engine

One method body is one value:

```rust
struct Forwarding {
    scrutinee: TokenStream,      // `self`, `*self`, `Pin::get_mut(self)`, or a tuple with the Self arguments
    arms: Vec<Arm>,              // one per selected target, cfgs carried
    fallback: Fallback,          // None | Mismatch(expr) | Body(block) | Unreachable(variants)
    future: bool,                // wrap in `async move { .. }`
}

struct Arm { cfgs: Vec<Attribute>, pattern: TokenStream, value: TokenStream }

struct Call {
    callee: Callee,              // Qualified { target_type, trait_path } | Method
    method: Ident,
    receiver: TokenStream,       // the payload expression handed over as `self`
    arguments: Vec<TokenStream>,
    awaited: bool,
    unsafety: bool,
}
```

Both entry points do the same three things: classify the signature into a
`MethodPlan` (receiver kind, same-variant arguments, return model, async,
unsafe), build a `Call` per selected target and run it through the return
model to get an `Arm`, and hand the `Forwarding` to the one renderer. What
differs between a trait method and an inherent method becomes data: the
`Callee`, which targets are selected, the `Fallback`, the `with = path`
lift, and two policies that are deliberate once named. In a trait method
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
struct ReturnModel { wrapper: Wrapper, value: Value }  // `future` travels beside it in the plan

enum Wrapper { None, Option, Result { error: Conversion } }
enum Value { Plain, Rewrap, Into }    // `Rewrap`: `Self`; `Into`: `Self::X` with `X` overridden
enum Conversion { Keep, Into }        // `Into` when the error is an overridden `Self::X`
```

Rendering composes: the inner conversion is the rewrap constructor or
`Into::into`, applied directly under `Wrapper::None` and through `.map(..)`
otherwise; `ErrorConversion::Into` adds `.map_err(Into::into)`. The
recognition rules stay syntactic and unchanged: `Option` and `Result` by
the name a path ends in, `Self::X` as a two-segment path, `impl Future` by
its `Output` binding, and any other `Self` in a return type is a limit.
`Result<Self, Self::Error>` with `Error` overridden then rewraps and
converts, and gets a behaviour test and an expansion golden.

### 3.4 The subject

`Subject` keeps its two cases and answers the questions the generators ask
instead of being asked what it is: the entries, the pattern prefix, whether
two values of it can hold different targets (an enum can, so a `Self`
argument needs a mismatch arm), whether it has conversions, and the shape
errors of its unforwardable entries. `from` and `try_from` move from
`Target` into `Place::Variant`, since only a variant has them. The subject's
carrier is named `__whichever_subject_..` for both cases.

### 3.5 The protocol

`protocol.rs` holds one struct per message, each with `ToTokens` for the
sender and `Parse` for the receiver, and the grammar in one place:

```
@v1 <facade> enum { <subject> } spec { <options> } [impl { <block> }]       sent by #[dispatch] to a trait's carrier
@v1 trait { <trait> } [remote { <path> }] <the message above>               what the carrier hands implement!
@v1 <facade> [partial { <options> }] impl { <block> }                        sent by #[dispatch] to a subject's carrier
@v1 [rejected] enum { <subject> } <the message above>                        what the carrier hands implement_block!
```

The trait's carrier is compiled into the trait's crate and read by whatever
version of this crate the subject's crate builds with, so the first two
lines are a compatibility surface: a segment is only ever added, optional,
and `v1` becomes `v2` when a segment changes meaning or order. The
subject's carrier never leaves its crate. `implement_inherent!` is renamed
`implement_block!`; the name `implement!` cannot change.

### 3.6 Options

`options.rs` owns every `#[dispatch(..)]` and `#[dispatchable(..)]` grammar,
one typed struct per position: the trait attribute on a subject, the
subject-level switches, a variant or field, an inherent method, a partial
block. The right-hand side of `Name = ..` stays a `TokenStream` until
`implement!` knows whether `Name` is an associated type or const, and is
parsed as a `Type` or an `Expr` then. Subject-level `from` and `try_from`
are read where the subject is expanded and are not sent through the
carrier. Each struct validates itself once; the three scattered checks go.

### 3.7 Carrier naming

The hash becomes FNV-1a over the trait's text and token positions, written
in the crate, so the name is stable by construction rather than by the
accident of one compiler per build. Token positions stay in the name, and
with them the 1.88 minimum, which the crate docs then state as the reason.

### 3.8 Diagnostics policy

Every generator collects errors with `syn::Error::combine` and reports all
of them; a rejected subject is still emitted, as today. A trait method's
parameter keeps its own name when its pattern is a plain identifier and gets
`__argument<n>` otherwise, so expansions read like the trait, and a
`mismatch` expression written on the trait method may name the arguments.
One written on the enum cannot: the trait's tokens reach the enum through
a `macro_rules!` body and carry its hygiene, while the enum's attribute
carries the enum's, which is also why the generated `match` reuses the
receiver's own `self` token.

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
for the return model, and a run on 1.88 as well as stable. The terminology
file's entry for the implementation macros still names
`implement_inherent!`; it is being edited outside this branch and gets the
new name when the two meet.

## 5. Invariants every stage keeps

- The generated impl is the one a careful hand writes: a `match`, one fully
  qualified call per arm, `#[inline]`, no conversion unless an option asks
  for one. `benchmarks/scripts/codegen.py` keeps checking this.
- Generated bindings use `Span::mixed_site()`; the receiver uses the trait
  method's own `self` token; `#[cfg]` is carried onto arms and methods.
- Nothing is kept between expansions.
- `implement!` keeps its name and accepts every `@v1` message a 0.x carrier
  emits.
- A rejected subject is still emitted, with its carrier, so the code around
  it compiles.
- The facade stays `no_std` and dependency-free; the minimum Rust version
  stays 1.88.
- Every change to a golden or a `.stderr` file is listed in the stage's
  status line.

## 6. Open decisions

- *Subject* is the internal name for the enum or struct and the benchmark
  word for a strategy under test. Keeping it is fine while it stays out of
  user-facing text; renaming it is a one-module change.
- Subject-level `from` and `try_from` on a trait attribute are accepted
  today. Stage 5 keeps accepting them; rejecting them is a breaking change
  for a later release.
- `proc-macro-crate` exists only so the facade can be renamed and brings
  eight build-time crates. A `renamed-dependency` feature, on by default, is
  the smallest way to let a user opt out.
- A trait parameter with two or more bounds leaves `Arg::Assoc` unqualified
  and rustc reports E0223. A whichever error naming the parameter is a
  small addition to `substitution.rs` once it has unit tests.
