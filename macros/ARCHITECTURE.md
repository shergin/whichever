# Architecture of `whichever-macros`

Status: landed. Written 2026-11-02 as a plan against 0.4.0 and carried out
the same day, stage by stage; a second round followed from two independent
reviews of the result. Sections 1 and 3 describe the crate as it is,
rewritten after the review of 2026-11-15 (stage 25), section 2 records what the shape was before, and each stage in section 4
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
| `#[dispatch(Trait, ..)]` on an enum or struct | the subject's crate | The subject is read into its entries and re-emitted without its `#[dispatch]` attributes, unless it restates another crate's enum with `remote = path`, in which case nothing is emitted for it. The conversions are written, and the shape errors of the entries no trait method can forward to. A crate-local carrier holding the subject's tokens is emitted and re-exported under the subject's name. For every trait named, `Trait! { @v1 facade enum { subject } spec { path, options } [remote { path }] }` is emitted. |
| The trait's carrier | expands in the subject's crate | Prepends `@v2 trait { signatures } [remote { path }] [provided { methods }]` and calls `::facade::__private::implement!`. |
| `implement!` | | Reads the carried trait into `Signatures`, the subject into its `Entries`, the trait's path and its `TraitOptions` apart, and an optional partial impl block into a `PartialBlock`; writes the forwarding impl, the agreement checks and the shape errors. |
| `#[dispatch]` on an impl block | the subject's crate | Emits `Subject! { @v1 facade [partial { options }] impl { block } }`. |
| The subject's carrier | | Prepends `@v1 [rejected] facade { facade } [traits { paths }] [remote { path }] enum { subject }` and calls `implement_block!`. |
| `implement_block!` | | An inherent block: reads its items once (`BlockItem`) and rewrites every marked method into a forwarding method with the body as the fallback. A trait impl block: re-emits `Trait! { .. impl { block } }`, so the trait's carrier and `implement!` finish the job. |

Inside `implement!`, `Subject::read` yields the entries: an `Entry` per
variant, or per field of a struct, holding a payload or another shape, the
second kept so that errors carry its `#[cfg]`. The reading has three more
steps, each done once before any target is taken: `Entries::for_trait`
keeps the one field of a struct the trait goes to, `Entries::rename` spells
the payloads with the names a partial block's header gives the subject's
parameters, and `Entries::route` gives the variants the trait's own routes.
`Impl` then holds what every item of the trait's impl is written against,
and `ItemWriter` plans each item of the trait and writes it. A method's
signature, with the trait's generics substituted by the arguments the
subject named, goes to `forward` with its `Origin`, which classifies the
method by receiver, by arguments and by return shape, judges each target,
and writes one `match` with a fully qualified call per arm. Associated
types and consts are taken from the first target and checked against the
rest in a `const _` block; the type check's helpers are associated
functions of a wrapper struct of the subject declared in that block, so
that the subject's parameters belong to their impl and the bounds its
fields imply hold there.

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
  lib.rs           crate docs; the four proc-macro entry points; `combined`; `mod` lines
  dispatchable.rs  `#[dispatchable]`: the trait's carrier, its method attributes checked
  dispatch.rs      `#[dispatch]` on a subject or an impl block: what is emitted at the attribute
  naming.rs        how the carrier macros are named: a fixed hash of the item's text and, for a trait, its token positions
  protocol.rs      the messages: one struct each, `ToTokens` and `Parse`, and the carrier builder
  options.rs       every `#[dispatch(..)]` and `#[dispatchable(..)]` grammar, one struct per position
  signatures.rs    `Signatures` and `Method`: the carried trait, read under either prefix
  subject.rs       `Subject`, `Entries`, `Entry`, `Place`, `Target`: the subject read into its entries
  route.rs         `Route`: how an arm reaches its target from the payload
  builds.rs        `Builds` and `Predicate`: the builds an item exists in
  site.rs          `Site` and `at`: where a generated token is written
  implement.rs     `implement!`: the forwarding impl of one trait, and `Impl`, its one context
  items.rs         `ItemWriter`: each item of the trait planned, then written against the `Impl`
  partial.rs       `PartialBlock`: a partial impl block as read
  marked.rs        `BlockItem` and `Marked`: an impl block's items read once for both paths
  block.rs         `implement_block!`: inherent blocks, and partial impl blocks handed to the trait's carrier
  forwarding.rs    the engine: `Origin`, `forward`, `MethodPlan` and `Forwarding`
  arm.rs           one arm: `ArmKind`, `ArmContext` and `Arm`, judged and written
  refusal.rs       `Refusal` and `WayOut`: why a target cannot take a method, and the sentence
  selection.rs     which targets a method's mark names, `OwnArms` under `*`, and the check on its body
  shape.rs         classifying receivers, arguments, return types and pointers
  spelling.rs      how a type, a path or a route is spelled in a sentence
  words.rs         `Words`: whether a sentence speaks of an enum and its variants or of a struct and its field
  substitution.rs  the trait's generics replaced by the subject's arguments, the subject's by a header's
  agreement.rs     the associated type and const checks
  conversions.rs   `From`, `TryFrom` and `into = Target`
  limits.rs        a test: every sentence the macros write is pinned by a `tests/ui` snapshot
  spans.rs         a test: no module that writes generated code writes at the call site
```

Stage 1 created the files by moving code; later stages changed what is in
them, and the review of 2026-11-15 added `builds.rs`, `site.rs`,
`signatures.rs`, `marked.rs`, `partial.rs`, `refusal.rs`, `arm.rs` and
`items.rs`, each holding a concept that had lived as booleans, strings or
token streams in the modules around it. `shape.rs`, `options.rs`,
`protocol.rs`, `substitution.rs`, `route.rs`, `spelling.rs`, `naming.rs`,
`dispatchable.rs`, `builds.rs`, `subject.rs`, `signatures.rs`,
`marked.rs`, `partial.rs`, `forwarding.rs`, `refusal.rs` and `words.rs`
have unit tests through `parse_quote!`, which the monolith made impractical.

### 3.2 The forwarding engine

A method comes to the engine from one of two paths, which say so:

```rust
enum Origin<'a> {                 // what a path hands to `forward`
    Trait { method: TraitMethod<'a>, mark: Option<Mark<'a>> },
    Inherent { mark: Mark<'a> },
}

struct TraitMethod<'a> {          // what a trait brings to one of its methods
    trait_path: &'a Path,         // every arm's call is qualified by it
    subject_path: &'a Path,       // the subject's name, or the path a restatement names
    declared: &'a Signature,      // the trait's, which a sentence about the method points at
    has_default: bool,
    block_header: &'a str,        // the partial impl block a sentence points at, spelled out
    options: &'a TraitOptions,
    mismatch: Option<Expr>,       // the enum's over the trait method's
    resolution: Option<Span>,     // where the names an enum-side `mismatch` uses resolve
    derived: bool,                // the subject attribute's impl, `#[automatically_derived]`
}

struct Mark<'a> {                 // a method's `#[dispatch(..)]`
    options: &'a ForwardingOptions,
    body: Option<&'a Block>,      // the arm for every variant the mark does not name, or under `*` the method's own arms
}

struct Forward<'a> {              // one method to forward
    signature: Signature,
    origin: Origin<'a>,
    entries: &'a Entries<'a>,
    subject: &'a Subject,
    site: Site,                   // the span the arms and the attributes carry
    body_site: Site,              // the method's own, on the body's braces
}
```

`forward` derives everything else from the origin and runs one sequence
for both paths. It names the `ArmKind`, the pattern prefix, the lift and
the `WayOut`; reads the signature into a `MethodPlan`; selects the targets
a mark names (`select_targets`, with `check_body`), or takes every entry;
writes an arm per target with `MethodPlan::arm`, and the arm for the rest
with `MethodPlan::fallback_for`; and renders the `Forwarding` into a
`Generated`: the signature as bound, the lint allowances a hand writes, and
the body.

Under `*` an enum's method leaves no variant to a body for the rest, so a
body there is the arms the method writes itself. `select_targets` takes the
mark's body and `takes_arms`, which `forward` passes as
`MethodPlan::matches_self_alone`: whether the `match` is `match self` over
the receiver alone, with no `Self` argument beside it and not `by_place`.
It reads the body with `OwnArms::read` into `Selection::own`, and
selects every entry no arm answers for in some build: `OwnArms::written` is
the `match` the body has to be, one `match self` and nothing else, and
`names` collects the variants an arm's pattern names by path at its top
level, through a guard, an or-pattern, parentheses, a reference or a
binding `name @ ..`, a lone identifier being the variant of that name where
the enum has one. `OwnArms::leaves` is the `Builds` in which no arm names a
variant, the complement of its arms' `cfg`s, so a variant whose arm is
under `#[cfg]` is forwarded where the `cfg` does not hold
(`Selection::forwarded_where`), and an entry without a payload gets the
arm holding its sentence only in those builds. Where `takes_arms` holds
the sentence ends in the arm a hand writes for it, with the pattern
`Unforwardable::rest` spells; where it does not, the method can take no
arm, and the sentence names the list with a body for the rest. `check_body` accepts
the body when `own` is there, and `MethodPlan::takes_own_arms` refuses a
`Self` argument beside the receiver and a `by_place` match, neither of
which is the `match self` the arms were written for. `forward` hands
`fallback_for` the body as `rest` only when it holds no arms of its own,
and the arms go into the `Forwarding` as `written`, each closed by a comma,
after the forwarding arms and before any fallback, inside the future where
the arms are awaited. `WayOut::own_arms` makes a refusal offer the arm of
the variant in the body, and the method as a whole "write every variant's
arm in the body".

```rust
struct MethodPlan {               // read off the signature and the entries once
    receiver: ReceiverKind,       // Value(Access) | Boxed | Pinned(Borrow)
    self_token: Token![self],     // reused, so the `match` shares its hygiene
    arguments: Vec<Ident>,        // the bindings after the receiver
    same_variant: Vec<(usize, Access)>, // the `Self` arguments, matched beside the receiver
    turbofish: Option<Vec<Ident>>,      // the generics a call spells out when inference could not
    conversion: Conversion,
    future: Option<FutureShape>,  // Opaque | Boxed(path): the `async move` block the arms sit in
    awaited: bool,
    unsafety: bool,
    requires_sized: bool,         // `where Self: Sized`, which a trait object payload cannot meet
    by_place: bool,               // `match *self` with `ref` bindings, over an enum that may be empty
    may_be_empty: bool,           // every entry under `#[cfg]`, so some build leaves no arm
}

enum ArmKind<'a> {                // what an arm does with its target
    QualifiedCall { trait_path: &'a Path, method: &'a Ident },
    MethodCall { name: &'a Ident },
    Place { field: Option<&'a Member>, borrow: PlaceBorrow },
}

struct ArmContext<'a> {           // what every arm of one method shares
    kind: ArmKind<'a>,
    method: &'a Ident,
    subject: &'a Ident,
    prefix: Path,                 // the subject's path, or `Self`
    lift: Option<&'a Path>,       // the inherent `with = path`
    way_out: WayOut,              // { on_trait: bool, has_body: bool, own_arms: bool }
    declared: Option<Declared<'a>>, // { signature, block_header }: a trait method's
    site: Site,
}

enum Refusal<'a> {                // why one target cannot take one method
    TraitObject { declared: &'a Signature, block_header: &'a str },
    RewrapThroughRoute,
    MethodRouteBorrowedMutably(Taken<'a>),
    SharedPointerBorrowedMutably(MutableBorrow<'a>),
    MovedOutOfPointer(Taken<'a>),
    NoBoxToHandOver,
    NotRebuilt,
}

struct Arm {
    attributes: Vec<Attribute>,   // the variant's `#[cfg]`, and `#[allow(deprecated)]`
    pattern: TokenStream2,
    value: TokenStream2,
    site: Site,
}

struct Forwarding {
    scrutinee: TokenStream2,      // `self`, `*self`, `Pin::get_mut(self)`, one of them behind `*`, or a tuple with the Self arguments
    arms: Vec<Arm>,               // one per selected target, plus the unforwardable ones
    written: TokenStream2,        // under `*`, the method's own arms as written, after the forwarding arms
    fallback: Option<Arm>,        // the mismatch arm, or the body for the rest
    future: Option<FutureShape>,  // wrap in `async move { .. }`, boxed again for a boxed future
}
```

The engine judges before it writes. `MethodPlan::refusal` returns the first
`Refusal` that holds for a target, a value that names what the method
takes and the target cannot give, as the signature has it (`Taken`,
`MutableBorrow`), in this order: a trait object beside `Self: Sized`, a
`Self` rewrapped through a route, a mutable borrow through a route ending
in a method call, a mutable borrow through a `Pointer::Shared`, a move out
of a pointer that is no `Box`, `self: Box<Self>` with no box to hand over,
and a `Self` the target cannot be rebuilt around. A move is a trait's
call, `ArmKind::QualifiedCall`, under `self` by value, which takes its
target as the signature says; a place under `self` by value or `self:
Box<Self>` that the return type does not borrow; and a `Self` argument
by value. An inherent call by name, `ArmKind::MethodCall`, is not judged:
it hands the target to a method of the payload's own, whose receiver
borrows or moves it as the signature here cannot say, so the arm is the
hand's, `(*inner).name()`, and rustc's to judge. `Refusal::error` words it
in `refusal.rs`, on the payload, with the way out the `WayOut` offers: an
impl by hand on the trait path, a body on the inherent path, the body
itself where the method has one for the variants it does not name, and an
arm of the variant in the body where the body holds the method's own
arms.
`MethodPlan::arm` then writes the target's arm, or the arm that holds the
sentence: the variant whatever it holds, `Target::whole`, with the
sentence as its result, under the variant's own attributes, so a build
without the variant forwards the rest and every variant the method cannot
reach is named. Writing an arm cannot fail. `MethodPlan::beside_arguments`
gives that arm, and the unforwardable ones the selection writes, the
tuple's shape where `Self` arguments are matched beside the receiver. An
entry without a target, a variant reported elsewhere under its own `cfg`,
gets its unforwardable arm, `Arm::unforwardable` with `standing_in` as its
result, beside `Self` arguments as well, though the arm for two different
variants already keeps that `match` exhaustive: that arm does not read the
other arguments, and in a build that leaves no forwarding arm nothing else
would.
What fails a whole method is what `forward` returns as an error: a
signature the plan cannot read, a mark naming a variant the subject
lacks or one without a payload, a body where none would run or none
where one must, a body under `*` that is no `match self` or an arm of it
that names no variant, arms of the method's own beside a `Self` argument
or over a `by_place` match, and a `Self` argument without a `mismatch`.
The trait path writes such a method with
the sentence as its body; the inherent path keeps the method as written,
without its mark, and `standing_alone` in `marked.rs` closes a body that
holds the method's own arms with `_ => ::core::unreachable!()` under
`#[allow(unreachable_patterns)]`, since those arms alone are not
exhaustive and rustc's E0004 would stand beside the error that kept the
method; the closing arm reads the arguments first, as `standing_in` does,
so that no lint about them stands beside that error where the only arm
using one is compiled out; and it makes the kept `match` look at what the
forwarding's would,
through `receiver_scrutinee` in `arm.rs`: `*self` under `self: Box<Self>`,
`::core::pin::Pin::get_mut(self)` or `get_ref(self)` under a pinned
receiver; `without_method_attributes` in `dispatch.rs` does the same for a
rejected inherent block, which is still emitted without its marks.
`MethodPlan::read` takes the entries the `match` is over: where every one
is under `#[cfg]`, so that some build may leave none, and the receiver is a
reference with no `Self` argument beside it, `by_place` makes the `match`
look at the subject itself, `match *self` with `ref` or `ref mut`
bindings, which a reference to an empty enum needs to be exhaustive; every
other method keeps `match self` with default binding modes, and
`may_be_empty` gives a block's method over such an enum the
`#[allow(unused_variables)]` a hand writes, where the impl is not
`#[automatically_derived]` and the method has no body for the rest.
What differs between a trait method and an inherent method is data on the
`Origin`: the `ArmKind`, which targets are selected, the fallback, the
`with = path` lift, the way out a sentence offers, and two policies that
are deliberate once named.
`MethodPlan::fallback` writes the arm for two different variants and binds
again a same-variant argument the scrutinee moved where the `mismatch`
expression names it, an identifier at any depth or a name inside a literal,
which a format string captures (`names`); one it does not name stays in the
scrutinee, as a hand leaves it. `MethodPlan::standing_in` is the result of
an arm that only keeps the `match` exhaustive beside a variant's own error,
and reads the arguments the `match` does not hold, so that no lint about
them stands beside that error: a `Self` argument is read by the scrutinee,
and one taken by value was moved into it, so the arm reads the others. Every token the engine writes carries the context's `Site`, the
subject's or the method's own span, the generated bindings and a
turbofish's angle brackets included: a proc macro's call site inside a
carrier's expansion is the trait's attribute, which may be in another
crate. In a trait method
`Self` is the implementor, so a `&Self` argument is matched on the same
variant and needs a mismatch; in an inherent method `Self` is the enum
itself, so a `&Self` argument is handed to the payload's method as it is,
and a payload method taking the enum is a legitimate target. A trait
method's parameter names come from a trait the enum's crate may not own:
each is kept where it is a plain identifier and generated otherwise; an
inherent method's are the user's, and a pattern that is not a plain
identifier is an error there.

Above the engine, `expand_implement` in `implement.rs` reads as the
sequence it is: the partial block, the header, the entries with the
header's names and the trait's routes, the bounds, the `Impl`, the items,
the checks. `Impl` is the one context of the trait's impl: the subject and
its path, the trait's path and name, the `Signatures`, the `TraitOptions`,
the entries, the `PartialBlock`, the block header spelled out, the
`supplier` of the associated items that are not overridden, and the site.
`ItemWriter` in `items.rs` plans each item of the trait before it writes
it: an item has zero or more `ItemPlan`s, none when the block writes it in
every build or leaves it to the trait's default, and one per way it is
written when the block writes or marks a method under `#[cfg]`. A plan is
a method rejected with its sentence as its body, an associated type or
const from the override or the supplier, a method forwarded to every
variant or as a mark names, or a limit; the plain forwarding, or a mark
without a `#[cfg]`, takes the builds the block's conditional items leave,
under the complement of their `cfg`s, a `Leftover`: a `Builds` and the
span its attribute carries. `Builds`, in `builds.rs`, is never, under a
`Predicate`, or every; a predicate is a `cfg` condition as written or
composed with `all`, `any` and `not`, and a composition of one predicate
is that predicate, as a hand writes it. `ItemWriter::plain_forwarding` is
the `Builds` in which some method with a receiver is forwarded to every
variant, and `Entries::shape_errors` reports a variant no method can
forward to under that `cfg` beside the variant's own, so a mark under the
trait method's own `cfg`, whose leftover holds in no build that has the
method, reports nothing. The routes a trait keys by variant are applied by
`Entries::route`, which returns the error of each route through no
variant, and `TraitOptions::unknown_names` returns the errors of the
options naming nothing in the trait.

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
wrapper is fine. `Conversion::rebuilds` says whether a value is built
around the payload's result, `whole`, and whether a path has to build it,
`by_path`, which the refusal checks against `Target::is_rebuilt` and
`Target::is_built_by_a_path`. The recognition rules stay syntactic: `Option` and
`Result` by the name a path ends in, `Self::X` as a two-segment path,
`impl Future` by its `Output` binding, a boxed future, `Pin<Box<dyn
Future<Output = T> + ..>>`, by the same binding under `Pin` and `Box`
(`FutureShape`), and any other `Self` in a return type is a limit, whose
sentence offers a provided method the trait's default first. An
opaque future is one `async move` block whatever the conversion; a boxed
one is wrapped, and pinned again with the signature's own `Box`, only to
convert its output. `Result<Self, Self::Error>` with `Error` overridden
rewraps and converts (golden `overridden_results`); `(Self, u32)` and
`(Self::Rest, Option<Self>)` take the tuple apart (golden
`tuple_returns`).

### 3.4 The subject

`Subject` keeps its two cases and answers the questions the generators ask
instead of being asked what it is: whether two values of it can hold
different targets (an enum can, so a `Self` argument needs a mismatch
arm), whether it has conversions, whether it has one target, and what a
sentence calls it: `Subject::words` answers `Words::Enum`, whose sentences
speak of the enum and its variants, or `Words::Struct`, whose sentences
speak of the struct and its field and never of a variant. The subject's carrier is named
`__whichever_dispatch_..` for both cases, after the attribute that writes
it.

`Subject::read` reads the subject once into `Entries`. An `Entry` is a
variant of the enum or a field of the struct, with its name, its `cfgs`
and whether it is deprecated whatever it holds, and what it holds:
`Holds::Payload { place, ty, route }`, or `Holds::Other(Unforwardable)`
for a variant of another shape, which carries the reason and the sentence.
A struct's field options are parsed once there, and what each field is
marked with stays beside the entries until `Entries::for_trait` keeps the
field a trait goes to, or `Entries::star` names the field `*` means.
`for_trait` serves both homes of a trait's impl alike, the subject's
attribute and a partial impl block's header, and compares the path as the
home spells it, so a field names a trait as its home does. Two functions
speak for a field `for_trait` left out. `Entries::spelt_otherwise` finds a
field that names a home's trait in another spelling, when no field names it
as spelled: `same_name` takes two paths for one name when the shorter ends
the longer and their generic arguments agree or one has none, so
`Catalog` and `traits::Catalog`, or `Encoder` and `Encoder<F>`, are one
name, and `Encoder<Json>` and `Encoder<Xml>`, or `a::Named` and
`b::Named`, are two. `Entries::forwarded_elsewhere` names the field the
trait is kept for when a partial block's method, or a trait's route, names
another field of the struct, so the sentence says where the trait goes
rather than that the struct lacks the field. The
other steps of the reading edit the entries before any target is taken: a
partial block's header names the subject's parameters, `Entries::rename`,
and a trait's routes, `via(Variant = ..)`, win over the variants' own,
`Entries::route`.

A `Place` is what holds the payload and where the payload sits in it.
`holder` is a `Holder::Variant`, with the variant's `Conversions`, or
`Holder::Struct`; `field` is a `PayloadField`: `Only`, the one field,
unnamed, which the name builds as a path; `OnlyNamed`, the one field,
named, which the braces build and no path does; or `Beside`, one field
beside others, around which no value is built. A struct with one named
field and a variant whose one field is named and marked are both
`OnlyNamed`, so `-> Self` is rebuilt by the braces for each, and a `Self`
under an `Option` or a `Result` is refused with the tuple form as the way
around (`Target::is_built_by_no_path`). Only a variant's `Only` field takes
part in the conversions (`Target::conversions`). `Target::pattern` writes a
rest pattern only for a `Beside` field that other fields follow,
`Beside { member, followed }` as `PayloadField::beside(member, count)`
builds it: `Tagged(inner, ..)`, `Middle(_, inner, ..)` or `Row { cell:
inner, .. }`, where a named field is always `followed`, since braces skip
the rest whatever their order. An unnamed last field is matched without
one, `Last(_, inner)`; an `Only` field is matched `Wrapped(inner)` and an
`OnlyNamed` one `Named { field: inner }`, on a variant and on a struct
alike, as a hand writes them.

A `Target` is a `Copy` view of an entry that holds a payload, `entry`,
`place`, `payload` and `route`, with the route this method takes: the
method's own through `Target::through`, or else the entry's, which is the
trait's for the variant or the variant's or field's own. No layer edits a
target. Since 2026-11-05 the route is a `Route`, the steps from the bound
payload to the value the trait is called on, where it was a `deref` flag:
`deref` is the route `*`, a field or a method is a step, and
`PayloadField::Beside` holds the marked field of a variant with several.
`Target::target_type` is `None` where the route leaves the type
unnameable, and the engine then qualifies the call by the trait alone,
`<_ as Trait>::method`. `Target::spoken` and `Target::noun` name the
target in a sentence: the variant, or on a struct the struct, whose field
the error's span points at. `Pointer` and `pointee` in `shape.rs` read a
payload's pointer kind and what it points at in one place: a `Box`, the one
pointer a value is moved out of; a shared pointer, an `Rc`, an `Arc` or a
`&T`, which lends what it points at shared only; or any other.

The inherent path's `ArmKind::Place` is the arm that calls nothing: the
target itself, or a field of it, handed over as `place_borrow` reads the
return type, inside the `Option` or the `Result` under `with = Some` or
`with = Ok`. Its `PlaceBorrow` is `Reference`, borrowed by `&` or `&mut`;
`Value`, taken by value; or `Unread`, left as it stands, for any other
lift, for one of the two over a return type that shows no first type
argument, for a raw pointer (`Type::Ptr`, to which a reference coerces)
and for an opaque return (`Type::ImplTrait`, which the reference may
implement); `ungrouped` looks through `Type::Group` and `Type::Paren`
first. Every target the pattern binds, a payload, a marked field or a
struct's field, is already a reference where the receiver binds references
(`MethodPlan::binds_references`), so it is read through the binding,
`*inner`, on a `Value` answer in an arm that is not awaited, and is the
binding on the other two.

### 3.5 The protocol

`protocol.rs` holds one struct per message, each with `ToTokens` for the
sender and `Parse` for the receiver, the one builder both carrier macros
are written by, and the grammar in one place:

```
request to a trait's carrier    @v1 <facade> enum { <subject> } spec { <path>, <options> } [remote { <path> }] [impl { <block> }]
what the carrier puts in front  @v2 trait { <signatures> } [remote { <path> }] [provided { <method>, .. }]
what `implement!` reads         @v2 trait { <signatures> } [remote { <path> }] [provided { .. }] enum { <subject> } spec { .. } [remote { .. }] [impl { <block> }]
                                @v1 trait { <trait> } [remote { <path> }] enum { .. } .., the prefix of the carriers 0.1 to 0.6 wrote

request to a subject's carrier  @v1 <facade> [partial { <options> }] impl { <block> }
what the carrier puts in front  @v1 [rejected] facade { <facade> } [traits { <paths> }] [remote { <path> }] enum { <subject> }
what `implement_block!` reads   @v1 [rejected] facade { <facade> } [traits { <paths> }] [remote { <path> }] enum { <subject> } [partial { <options> }] impl { <block> }
```

The options travel typed: a `TraitRequest` carries the trait's path and
its `TraitOptions` apart, a `BlockRequest` the partial block's options,
parsed and checked at the attribute. `ImplementInput` hands `implement!`
the `Signatures`, the subject, the subject's real path, the trait's path,
the options and the partial block, and `BlockInput` hands
`implement_block!` the carried subject and the request whole. The trait's
carrier is compiled into the trait's crate and read by whatever version of
this crate the subject's crate builds with, so the request to a trait's
carrier and the prefix it puts in front are a compatibility surface frozen
into every published trait crate. Between two keywords a message is a run
of segments, `tag` or `tag { .. }`; a segment is only ever added, and the
readers skip a tag they do not know, so an older reader accepts a newer
prefix of the same version, which a unit test and a behaviour test with
carriers of the 0.4.0 and 0.6.0 shapes pin. The version changes when a
segment changes meaning or order: the carried trait's prefix is `v2` since
2026-11-06, when the carrier came to hold signatures only, and `implement!`
reads `v1` and `v2`; the request stays `v1`, which every carrier ever
written matches, and every carrier's last rule reports any other request as
newer than the carrier. A reader too old for a prefix says so in one
sentence, which `smoke/mixed` holds against the last published version.
The subject's carrier never leaves its crate and forwards the facade's
name, so a partial impl needs no second lookup, and the traits the subject's
own attributes dispatch, so a partial block of one of them is reported as
the second impl it would be. The facade pins the macros
crate exactly: `__private` and the generated code are one interface.
`implement_block!` was `implement_inherent!` before stage 4; the name
`implement!` cannot change.

### 3.6 Options

`options.rs` owns every `#[dispatch(..)]` and `#[dispatchable(..)]` grammar,
one typed struct per position. On a subject the attribute is a
`DispatchAttribute`: the trait's path, when it names one, the trait's
`TraitOptions`, and the `SubjectOptions`. `TraitOptions` holds the
`overrides`, the `mismatch` results and the `routes`; it is what travels to
`implement!` beside the trait's path, which the request carries itself,
and what a partial block's attribute holds, whose grammar refuses a trait
path and a subject's option. `SubjectOptions` holds the subject's `from`,
`try_from`, `into` and `remote`, each with the span it was written at, read
where the subject is expanded and never sent. Who reads a list is a
`Reader`, which decides the words of its sentences:
`DispatchAttribute::read` reads the subject's own attribute as
`Reader::Subject` with the subject's `Words`, so an unknown option lists a
struct's options on a struct and an enum's on an enum, and a partial
block's attribute is read as `Reader::Block`, whose sentences name neither
and list the trait's options, the ones a block takes. `TargetOptions::read` reads
a variant's or a field's attributes at a `Position`: on a variant a route
and the variant's say on the conversions, on a variant's field a route
alone, `deref` or `via`, and on a struct's field a route and the traits the
field is the target of. A method's mark is a `ForwardingOptions` in an
inherent block and a `TraitMark` in a partial block: `default`, or the
variants and `via`, with the inherent options `call`, `with`, `field` and
`value` refused; both are read in the subject's `Words`, since the carrier
has brought the subject by then, so a struct's bare mark asks for its
field. `method_mismatch` reads a trait method's `mismatch = ..`
and `DispatchableOptions` a trait's `remote` and `name`.
The right-hand side of `Name = ..` stays a `TokenStream` until `implement!`
knows whether `Name` is an associated type or const, and is parsed as a
`Type` or an `Expr` then. The grammar rejects an option or a variant given
twice; `TraitOptions::unknown_names` checks the options against the
trait, beside their grammar, and rejects an override or a `mismatch`
naming nothing in it; `#[dispatchable]` rejects a method attribute that is
not one `mismatch = ..` on a method with a `Self` argument.

### 3.7 Carrier naming

The hash is FNV-1a over the trait's text and token positions, written out
in `naming.rs`, so the name is stable by construction rather than by the
accident of one compiler per build. Token positions stay in the name, and
with them the 1.88 minimum.

### 3.8 Diagnostics policy

A field's mark for a trait that no home implements is not reported: the
struct's attribute does not see the partial blocks, which are other
expansions, so a trait a field names that the attribute does not is taken
for a block's, and the trait's first use says when the impl is missing.
What can be told from one expansion is: one name spelled two ways, reported
at the struct's attribute on the field's mark (`expand_subject`) and at a
partial block on its trait path (`expand_partial`), through
`Entries::spelt_otherwise`.

A sentence speaks of the item the user wrote. It is written where the
subject is known, at the subject's own attribute and inside `implement!`
and `implement_block!`, and takes its words from `Subject::words`: an
enum's sentence names its variants, a struct's its field. The attribute of
an impl block is read before the subject's carrier brings the subject, so a
sentence written there (`Reader::Block`) names neither an enum nor a
struct.

Every generator collects errors with `syn::Error::combine` and reports all
of them, and every generator emits the item. A trait method that cannot be
forwarded is written with its signature and its sentence as its body, a
`compile_error!` rustc treats as an error expression and type-checks no
further, so the impl is complete, the calls around it resolve, and the
sentence stands alone. A method that cannot be forwarded to one variant
keeps its other arms and has the sentence as that variant's arm, under the
variant's own `cfg`, as a variant of another shape is reported under its
own: a build that compiles the variant out has neither, and each such
variant gets its sentence. An associated item no variant can supply holds
its sentence as what it is: a const its value, `{ compile_error!(..) }`,
and a type an array whose length is the error, `[(); { compile_error!(..);
0 }]`, which rustc takes as an erroneous type and checks nothing against.
What withholds the impl is what leaves no impl to write, and it is this
list. In `expand_implement`: a struct whose fields choose no one field
for the trait, or two (`Entries::for_trait`); an enum with no variant;
an enum whose every variant is rejected, each with its own sentence, which
an impl with nothing to forward to would only add rustc's errors to, so
that a partial block's impl is then the shape errors alone; and a trait
path whose generic arguments do not fit the trait's parameters, or bind an
associated item inside the path (`Substitution::of_trait`). In
`PartialBlock::read`, through `read_item`: a method of the block without
a body and without a mark, a macro invocation, whose items cannot be told
from the ones to forward, a mark that does not parse or names an inherent
method's option, a second mark on one method, and `#[dispatch(default)]`
on a method with a body. Such an item under `#[cfg]` never reaches it:
`expand_partial` reads each item with `read_item` first
(`without_unread_conditional_items`), takes one that cannot be read and is
not of every build out of the block, writes the trait's carrier invocation
under the complement of their `cfg`s, so the block is read without them
where none holds, and writes each sentence under its item's own `cfg`, so
every such item has its sentence where its `cfg` holds. In `ItemWriter::write`: a macro invocation inside the trait, an
associated type or const the block writes under `#[cfg]`, an override that
does not read as a type or as a value, and a signature or a const's type
the substitution could not write, a projection it could not qualify, which
leaves no signature to hold an error. An `ItemWriter` error is reported
with the errors that would have stood beside the impl, an option naming
nothing in the trait and a route through no variant, so that none waits
for the next build; the ones before it return alone. A
method over an enum whose every variant is under `#[cfg]`, in an impl that
is not `#[automatically_derived]`, a partial block's or an inherent
block's, allows its arguments to go unused, as a hand's does, since a
build that leaves no variant leaves no arm to use them; a method with a
body for the rest has that body as the arm, and its lint is the user's. An option naming nothing in the trait, or a route through no
variant, is reported beside the impl, which is right without it, and a
`#[dispatch(default)]` on a method without a default is written with the
sentence as its body. The inherent path keeps the block and the methods it could
forward, for the same reason, and a method it cannot forward stays as
written, without its mark, or is dropped when it has no body; its sentence
stands under the method's own `cfg` attributes, written beside the block
in `expand_inherent`, so a build that compiles the method out has neither,
as it would have no hand-written method. Tokens that are no method are
reported in every build. At the subject the unit of rejection is the
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
included, which the fallback arm binds again where the expression names it,
and a `&self` receiver. The
two live in different hygiene contexts, the trait's tokens having come
through a `macro_rules!` body, so the engine resolves the receiver's `self`
token and the written argument bindings at the chosen expression's span
with `Span::resolved_at` (stable since 1.45); the arms use that same `self`
token. A rustc error inside generated code is accepted only where a
hand-written impl gets the same error on the same user token, a variant's
payload or a trait method; one on an attribute is a defect. So every token
that a module writing generated code writes carries the subject's, the
method's or the payload's span, through a `Site`: a function that writes
generated code takes one, and only `Site::at` in `site.rs` makes one, the
hygiene of `Span::mixed_site()` located at a token the user wrote, so a
span that was never located cannot reach it. A generated binding,
`__inner`, `__same<n>`, `__argument<n>` or `__element<n>`, is built by
`hygienic` at the arm's `Site`: one located at the call site would join
the span of the expression that holds it to the attribute, and rustc's
error with it. The type holds the rule where a function takes a `Site`;
the unit test in `spans.rs` holds the rest: it reads those modules and
names a bare `quote!` or `parse_quote!`, which writes at the call site,
inside `implement!` the trait's attribute, and an identifier or a lifetime
built at `Span::mixed_site()` or `Span::call_site()` without a location.

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

Status: landed on 2026-11-06 on the branch `execute-review-2026-11-06`, one
commit per item in the order of `notes/review-2026-11-06.md` section 8, and
merged the same day. Every
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
A defaulted trait parameter is filled from its default.
One `trait_key`, the path as written, tells traits apart at every comparison.
The carried trait's prefix is `v2`, the reader reads `v1` too, every
carrier ends in a catch-all rule, and `smoke/mixed` builds the two-version
graph (`carrier_newer_than_the_reader` and `carrier_older_than_the_request`
are new; the older-release test gained the 0.6.0 shape).
An unknown override or `mismatch` name and a route through no variant are
reported beside the impl (`unknown_override_keeps_the_impl` is new).
A struct's inherent block sees every field, `*` the bare or only one
(`star_on_a_struct_with_marked_fields` is new; `struct_without_marked_field`
names the trait now).
A by-value move through a pointer that is not a `Box` is one sentence
(`route_moves_through_a_shared_pointer` is new).
A trait-side `mismatch` path resolving at the enum is documented
(`mismatch_path_resolves_at_the_enum` is new).
Floats joined the const check.
The carrier name's file key is relative to the manifest directory (no
golden changed: their files sit at `src/main.rs` of a generated crate,
where the two keys agree).
The target selection moved from `block.rs` into `selection.rs`, so the
trait path and the inherent path both sit on the engine and neither on the
other.
`expand_implement` plans each item before writing it (`ItemPlan`,
`ItemWriter`), and `forward_trait_method` asks `fallback_for` and
`check_trait_objects`; no golden or snapshot changed. `expand_implement` is
207 lines where it was 311 and `forward_trait_method` 163 where it was 184;
`conversion_impls` (184) and `expand_subject` (124) are the next seams.
`value` borrows a routed place as the return type asks, so it and `field`
are one arm (behaviour test `routes` extended; no golden changed).
A boxed future's output is converted inside a `Box::pin(async move { .. })`
(golden `boxed_futures` and the `async_trait_above_dispatchable` case are
new; `docs/decisions/a-boxed-future-is-rewrapped-by-boxing.md`).

### Stage 17: the search of 2026-11-12

Status: landed on 2026-11-12. `ReturnSite` carries the trait's default a
provided method can be left to, and `self_limit` names it before the other
ways out (snapshot `provided_self_wrapping_combinator` is new; no other
snapshot or golden changed; the behaviour test
`combinators_left_to_the_default` is new). `whichever::foreign` gained
`DoubleEndedIterator` and `ExactSizeIterator` (the behaviour test
`double_ended_and_exact_size_iterators` is new). The behaviour test
`pinned_writers_with_provided_methods` is new and pins what a restatement
forwards, with and without its provided methods.

### Stage 18: the top-four surveys' bugs

Status: landed on 2026-11-12. A mark in a partial block under `#[cfg]` is
planned as a method the block writes under one: `ItemWriter::plans` gives
it the builds its `cfg` holds in and the plain forwarding, or a mark
without a `#[cfg]`, the builds the block's conditional items leave, so
`#[dispatch(default)]` under a `cfg` is no longer the default in every
build and a second mark of one name no longer replaces the first. Golden
`conditional_partial_mark` and compile-fail case
`block_default_without_a_default_under_cfg` are new; no other golden or
snapshot changed; the behaviour test `conditional_methods_in_a_partial_block`
is extended, a struct's block included. `place_borrow` reads the borrow of
an inherent method's place through `with = Some` and `with = Ok`, from
the return type's first type argument, and any other lift keeps the place
by value; it has unit tests of its own. Golden `place_under_a_lift` is
new; the behaviour test `routes` is extended; no other golden or snapshot
changed. `MethodPlan::over` matches an enum whose every variant is under
`#[cfg]` by place where the receiver is a reference, so a build that
leaves no variant is exhaustive, and every other enum expands as before.
Golden `every_variant_under_cfg` and the behaviour test
`enums_with_every_variant_under_cfg` are new; no other golden or snapshot
changed (`docs/decisions/an-enum-that-may-be-empty-is-matched-by-place.md`).

### Stage 19: the acadrust survey's bug

Status: landed on 2026-11-13. `place_borrow` answers three ways through
`PlaceBorrow`, which `Callee::Place` carries in place of a nested
`Option`, and the place branch writes `*` for a marked field of a variant
with several on a `Value` answer under a receiver that binds references,
in an arm that is not awaited, so `value` on acadrust 0.6.8's `Unknown {
handle, .. }` reads the `Handle` its `-> Handle` asks for. Golden
`marked_field_by_value` is new; the behaviour tests `routes` and
`enums_with_every_variant_under_cfg` are extended; `place_borrow`'s unit
tests are rewritten for the three answers; no other golden or snapshot
changed.

### Stage 20: one reading for every place

Status: landed on 2026-11-13. The place branch of `MethodPlan::arm` no
longer asks `Target::is_marked_field`, which is removed: a plain payload
and a dispatch struct's field are read through the binding on a `Value`
answer as the marked field is, so `value` on `fn handle(&self) -> Handle`
is `*inner` for every target the pattern binds. `place_borrow` reads the
return type without a lift as well, answers `Unread` for `Type::Ptr` and
`Type::ImplTrait`, and looks through `Type::Group` and `Type::Paren`
(`ungrouped`). Golden `payload_and_field_by_value` is new, and golden
`marked_field_by_value` is regenerated over a `u64` handle, without the
derive whose expansion follows the toolchain, its arms unchanged; the
behaviour tests `routes`, `inherent_block_on_a_struct` and
`enums_with_every_variant_under_cfg` are extended; `place_borrow`'s unit
tests gain the pointer, opaque and grouped cases; no other golden or
snapshot changed
(`docs/decisions/a-place-is-handed-over-as-the-return-type-asks.md`).

### Stage 21: a place under `self: Box<Self>`

Status: landed on 2026-11-13. The place branch of `MethodPlan::arm`
follows a route through the binding's reference where
`MethodPlan::binds_references` says the pattern binds one, where it asked
for any receiver but `self`, so under `self: Box<Self>`, whose `match
*self` binds by value, `value` on a `deref` variant is `*inner` where it
was `**inner`, and `field = name` is `(*inner).name`. The behaviour test
`pinned_and_boxed_receivers` is extended; no golden or snapshot changed.

### Stage 22: the sentence for a place moved under `self: Box<Self>`

Status: landed on 2026-11-13. The check in `MethodPlan::arm` that refuses
a move out of a pointer that is not a `Box` counts a `Callee::Place` under
`ReceiverKind::Boxed` as owned unless its `PlaceBorrow` is `Reference`,
and the sentence names the receiver, `self: Box<Self>` where it said
`self` by value. Compile-fail case
`place_moves_through_a_shared_pointer_under_a_boxed_receiver` is new; the
behaviour test `pinned_and_boxed_receivers` is extended; no other snapshot
or golden changed.

### Stage 23: sentences spell their tokens

Status: landed on 2026-11-13. `spelling.rs` holds `spelled`, which the
nine sites that put a type, a path or a route into a sentence use where
each stripped every space from the tokens' `Display`, so `&'a Name` no
longer reads `&'aName`. `trait_key` is spelled by it too, so the
comparison key and the name in a sentence stay one string. Compile-fail
case `sentence_spells_a_type_with_its_spaces` is new; no existing snapshot
or golden changed; `spelled` has unit tests.

### Stage 24: the conformance matrix's bugs

Status: landed on 2026-11-14, unreleased, on the branch `matrix-findings`:
five fixes, four of defects the conformance matrix found and one of the
mp4-atom survey's that the matrix showed wider, each held by lines of
`known.tsv`, and the tests, docs and mutant of each in a commit of its
own.

`hygienic` takes the arm's site and builds at `at(site)`, and so does
`__value` in `Target::constructor`; `shape::turbofish` returns the
parameters, and the arm writes the angle brackets at its own site. A
rustc error in an arm, E0596 through a `deref` over `Rc`, E0308 or E0277
through a route ending in a method call or a `with` lift, lands on the
enum's name or the inherent method's, where its span ran from the
attribute (`e8014fe`). The test in `spans.rs` names an identifier or a
lifetime built at the call site. Compile-fail case
`mutable_borrow_through_a_shared_pointer` is new; no golden or snapshot
changed. `known.tsv` loses its thirteen `@attribute` lines: `s0=` and
`s1=` with `deref-rc`, `deref-arc` and `deref-ref` (E0596),
`s0=via-method`, `s0=via-method-value` and `s1=via-method` (E0308),
`s0=via-method` and `s1=via-method` (E0277), `lift=by-ref` and
`lift=by-value`.

`MethodPlan::arm_or_sentence` writes a target whose arm fails as the arm
carrying the sentence, under the variant's `arm_attributes`, where the
error failed the whole method, and `beside_arguments` gives it, and the
selection's unforwardable arms, the tuple's pattern beside `Self`
arguments (`4af7dbc`). Compile-fail cases
`deref_with_self_return_under_a_cfg_that_holds` and
`star_over_variants_without_a_payload_beside_a_self_argument` are new;
snapshot `route_moves_through_a_shared_pointer` gains the sentence for
`Borrowed`; no golden changed; the behaviour test
`cfg_on_variants_and_methods` is extended. `known.tsv` loses
`s1=unit path=block-star` (E0308), the twelve `cfg=second-off` and
`cfg=first-off` sentence lines and `cfg=first-off missing:sentence`.

`MethodPlan::fallback` binds a moved `Self` argument again only where
`names` finds it in the `mismatch` expression, and the arm beside a
variant's own error is `MethodPlan::standing_in`, which reads the
arguments, where it was `unreachable!()` alone (`73d350b`). Compile-fail
case `block_over_a_variant_without_a_payload_alone_in_the_build` is new;
no golden or other snapshot changed; the behaviour test
`mismatch_with_moved_arguments` is extended with a partial block under
`#[deny(unused_variables)]`. `known.tsv` loses the four
`warning:unused_variables` lines of `arg=value` and `arg=mut` under
`path=block` and `path=block-star`, and `path=block cfg=first-off`.

`Substitution::of_trait` keeps a single bound through `qualifier`, which
drops the constraints on associated items from its last segment's
arguments (`49df636`). A unit test beside the projection's own holds it,
a real generic argument kept; the behaviour test `generic_traits` is
extended; no golden or snapshot changed. `known.tsv` loses
`generic=concrete qual=provided` (E0229).

`ItemWriter::plain_forwarding` is a `PlainForwarding`, and a plan's
complement a `Leftover`, so `implement` writes `shape_errors` under
`#[cfg(any(..))]` of the leftovers where no plain forwarding holds in
every build (`822f043`). Compile-fail case
`conditional_mark_leaves_builds_to_the_forwarding` is new; no golden or
other snapshot changed; the behaviour test
`marks_under_the_trait_methods_cfg` is new. `known.tsv` loses the four
`item=cfg-on` lines, `s1=unit` and `s1=pair`, each on `variant-1` and
`variant-1-cfg`.

The mutants `bindings-at-the-call-site`, `refused-arm-without-its-cfg`,
`fallback-rebinds-every-moved-argument`, `projection-keeps-the-constraints`
and `shape-report-in-every-build` revert the five fixes, and each is
killed.

Five more changes followed on the same branch, two from the sentences the
matrix showed and three the owner chose on the matrix's questions, each
with a decision record. None added or deleted a line of `known.tsv`; the
model learns their sentences in its own change.

`MethodPlan::taken` names the `Self` argument a sentence is about, and the
move and method-route sentences name the receiver or the argument as the
signature has it; `receiver_expression` reports `self: Box<Self>` on a
`deref` variant over another pointer in a sentence of its own, where it
advised a `Box` of that pointer; `ArmContext::has_body` makes
`way_out` and `rebuild_hint` offer the body of a method that names its
variants (`a615da8`). Compile-fail cases `box_self_on_a_shared_pointer`,
`self_argument_moves_through_a_shared_pointer` and
`route_method_under_a_pinned_receiver_or_a_mut_argument` are new; snapshot
`sentence_spells_a_type_with_its_spaces` moved, its four sentences ending
"or leave `<variant>` to the body"; no golden changed. The mutants
`method-route-limit-dropped` and `move-limit-on-box` are rewritten for the
moved text, making the change each made before.

Under `self` by value a place counts as moved unless its `PlaceBorrow` is
`Reference`, as under `self: Box<Self>` (`f7949a3`). The behaviour test
`inherent_methods_on_dereferenced_payloads` is extended; no golden or
snapshot changed, `route_moves_through_a_shared_pointer` included.

`unknown_names` ends the sentence of a `mismatch` naming no method with
where a method under a `#[cfg]` takes its mismatch (`09f24e7`, the record
"An option names an item of every build"). Compile-fail case
`mismatch_for_a_method_under_a_cfg` is new, with both forms of the hint;
snapshots `unknown_option_names` and `unknown_override_keeps_the_impl`
moved; no golden changed.

`lends_only_shared` recognises an `Rc`, an `Arc` and a `&T`, and a
method that borrows its target mutably through one, by its receiver, a
`&mut Self` argument or a place returned as `&mut`, is a sentence on the
payload (`59dab88`, the record "A borrow a shared pointer cannot lend is
a sentence"). Compile-fail cases `mutable_argument_through_a_shared_pointer`,
`mutable_place_through_a_shared_pointer` and
`route_method_result_does_not_fit_the_receiver` are new, the last the pin
of where a rustc error in an arm lands that
`mutable_borrow_through_a_shared_pointer` was before its snapshot moved to
the sentence; the behaviour test `inherent_methods_on_dereferenced_payloads`
is extended with a shared borrow under `&mut self`; no golden changed.

`Target::constructor` writes no closure: a struct whose one field is named
has no path to `map` with, and `Target::is_built_by_no_path` gives the
rebuild error its sentence, with the tuple struct as the way around
(`5c80735`, the record "A named field is not rebuilt under a wrapper").
Compile-fail case `named_field_under_a_wrapper` is new; the behaviour test
`newtype_delegation` is extended; no golden or other snapshot changed.

`Target::spoken` names the target in a sentence: the variant, or on a
struct the struct, whose field the error's span points at, and
`Target::noun` says variant or field in the `Box` advice (`c929504`). A
`Self` return beside the other fields of a marked variant named the enum,
and a struct's field was `_0`; the conformance matrix's check of a
sentence's words found both. Compile-fail cases
`self_return_beside_the_other_fields_of_a_variant` and
`box_self_on_a_plain_field` are new; no golden or other snapshot changed.

The mutants `box-self-advice-on-a-shared-pointer`,
`borrowed-place-moved-under-self`, `absent-method-hint-dropped`,
`mutable-borrow-left-to-rustc`, `named-field-rebuilt-with-a-closure` and
`sentence-names-the-field-of-a-struct` revert these six.

An outside review of the matrix's model, written without reading this
crate's sources, found three more. An associated item no variant can supply
withheld the impl, against rule 4; `associated_type` and `associated_const`
now write it holding its sentence (`2e6bdc2`; compile-fail case
`unsupplied_items_keep_the_impl`). A partial block's or an inherent block's
method over an enum that may be empty warned of its arguments in the build
that leaves no variant; `MethodPlan::allow_unused_when_empty` gives such a
method the allowance a hand writes, where the impl is not
`#[automatically_derived]` (`19910b9`; the behaviour test
`enums_with_every_variant_under_cfg` denies the lint). A whole-method
sentence told a method with a body to give it one;
`ArmContext::whole_way_out` offers the body for every variant instead
(`0d7cc02`; compile-fail case `lift_over_a_wrapped_self`, which also pins
the lift over a wrapped `Self` as the limit `docs/limits.md` now names). No
golden and no other snapshot changed. Mutants
`unsupplied-item-withholds-the-impl`, `empty-enum-arguments-left-unused` and
`whole-method-way-out-ignores-the-body` revert them.

The matrix's later axes found six more. The const agreement checks were
one `const` block that stopped at its first failure; each is now an item
of its own (`4803ac0`; compile-fail case `const_disagreement_on_two_variants`).
An enum with lifetime parameters alone has its consts checked with each
lifetime `'static` (`fd733f6`; the record "Associated consts are checked
unless a type can change them"). A block method with a body for the rest
gets no allowance over an empty enum, its body being the user's
(`52ed853`; `block_body_keeps_the_users_lint`). `trait_object_refusal`
gives the sentence for a trait object beside a `Self: Sized` method as
that variant's arm, through `MethodPlan::sentence_arm`, where
`check_trait_objects` failed the whole method (`a64866e`;
`trait_object_beside_a_plain_variant`). A generic associated type the enum
does not declare is written holding its sentence (`8dc025d`;
`undeclared_gat_keeps_the_impl`). `select_targets` takes a struct's `*`
field from the entries the header renamed (`287953a`), and
`Substitution::of_subject` locates a type put in a parameter's place where
the parameter was (`5946f83`; `instantiated_variant_keeps_its_sentence`).
Snapshots `associated_const_disagreement` and
`const_disagreement_on_a_lifetime_enum` moved by the item rustc names; the
golden `associated_consts` shows the check as an item; no other golden or
snapshot changed. Each has its mutant.

### Stage 25: the review of 2026-11-15

Status: landed on 2026-11-15, unreleased, on the branch
`claude/library-implementation-quality-c12a43`, one commit per item. A
design review found that the vocabulary had types for half of its
concepts, and that the other half, the builds an item exists in, a
sentence and its way out, where a method comes from, the pointer kind of a
payload, lived as booleans, strings and token streams. Twelve commits gave
each a type and changed no expansion; five changed what is reported, each
in a commit of its own.

`spelling::spelled` spells the block header, where `implement.rs` had a
copy of its own (`9c29db9`). `Builds`, never, under a `Predicate` or
every, holds the builds a block's items are written in, the builds they
leave and the builds some method forwards to every variant in, which were
`Written`'s variants, `Leftover`'s predicate tokens and `PlainForwarding`,
with two copies of the `cfg` extraction; a composition of one predicate is
that predicate (`2d187d1`). `Site`, made only by `Site::at`, is what a
function writing generated code takes, and `at` moved from `lib.rs` into
`site.rs` (`cbc89ab`). `Subject::read` returns `Entries`, an `Entry` has
its name, `cfg` and deprecation whatever it `Holds`, a `Place` is a
`Holder` and a `PayloadField`, a `Target` is a `Copy` view with the route a
method takes, a trait's routes and a header's names are steps of the
reading, and a struct's field options are parsed once (`67a4fd7`); `shape.rs`
gained `Pointer` and `pointee`. `TraitOptions` is apart from the trait's
path, which `TraitRequest` and `ImplementInput` carry themselves
(`f00343d`). `Signatures` reads the carried trait under either prefix, and a
`Method` answers `has_default` and `may_be_left_to_default` through
`DefaultBody` (`83666ca`). `TraitOptions::unknown_names` checks the options
beside their grammar (`90699a1`). `BlockItem::read` reads an impl block's
items once for both paths, `PartialBlock` is the partial block as read,
where `ImplHeader` was, and `TraitMark` is the grammar of a partial block
method's mark beside the others (`d980324`). `MethodPlan::refusal` judges a
target and returns a `Refusal`, `Refusal::error` words it with the
`WayOut`, `MethodPlan::arm` writes the arm or the sentence's and cannot
fail, `ArmKind` is what `Callee` was, `Origin` is what the two paths hand
to `forward`, which runs the one sequence, `MethodPlan::read` takes the
entries where `MethodPlan::over` read them, and `ReceiverKind::Pinned`
holds a `Borrow` (`e168e81`). `Impl` is the one context of a trait's impl
and `ItemWriter`, in `items.rs`, plans and writes the items against it
(`9f4fb1a`). `expand_partial` and `expand_inherent` take the `BlockInput`
whole (`d3d5d1d`). The module doc of `naming.rs` says how a subject's
carrier is named (`a6719fc`), and `SubjectOptions` is what `Switches` was
(`26c358a`).

The five that change what is reported: an item that withholds the impl is
reported with the options naming nothing in the trait and the routes
through no variant, which waited for the next build (`fb6b0ab`); a second
`#[dispatch(..)]` beside `#[dispatch(default)]` on a partial block's method
is reported, where it was ignored (`d980324`); a trait named on a variant's
field is one sentence, `Position::VariantField` having a grammar of its
own, where it was accepted and ignored (`7297e45`); a variant whose one
field is named and marked is `PayloadField::OnlyNamed`, rebuilt by its
braces for `-> Self` as a struct with one named field is, and under
`Option<Self>` or `Result<Self, E>` its sentence names the tuple variant,
where it was a field beside others and refused with a sentence about other
fields (`8002106`, the record "A variant's one named field is rebuilt by
its braces"); and a macro invocation in a partial impl block is one
sentence, where it was dropped and the forwarding stood in its place
(`554eb64`). Compile-fail cases `withheld_impl_keeps_the_errors_beside_it`,
`block_default_marked_twice`, `trait_named_on_a_variants_field`,
`self_in_an_option_over_a_variants_named_field` and
`macro_in_a_partial_block` are new; golden `named_field_variant` and the
behaviour test `rebuilt_named_field_variants` are new; `builds.rs`,
`subject.rs`, `forwarding.rs`, `refusal.rs`, `marked.rs`, `partial.rs`,
`signatures.rs`, `options.rs` and `shape.rs` gained unit tests, and
`spans.rs` reads the new engine modules. No existing golden or snapshot
changed. The conformance matrix agrees on every cell, and no line of
`conformance/known.tsv` was added or deleted; the mutants are retargeted
at the moved code, none added or deleted.

### Stage 26: a sole field is matched without a rest pattern

Status: landed on 2026-11-15, unreleased, on the branch
`sole-field-pattern` (issue #11). `Target::pattern` matched a struct's one unnamed field as `Wrapped(inner,
..)` and the one named field of a struct or a variant as `Named { field:
inner, .. }`, a rest pattern with nothing to skip that no hand writes. It
now writes `Wrapped(inner)` and `Named { field: inner }`, and asks only the
`PayloadField`, since the holder no longer changes the pattern; a `Beside`
field keeps its `..`. Goldens `generic_payloads`, `named_field_variant`,
`newtype` and `opaque_through_a_struct` moved, each line a sole field's
pattern losing its `, ..`; goldens `per_trait_fields`, `routes`,
`marked_field_by_value` and `payload_and_field_by_value` match only fields
beside others and did not change. The unit test of `Target::pattern` in
`subject.rs` is updated for the three sole-field cases. No snapshot
changed, and nothing changes at run time.

### Stage 27: arms under `*`

Status: landed on 2026-11-15, unreleased, on the branch `arms-under-star`
(issues #5 and #8), the record "Under `*`, a method's body holds the enum's
own arms". A hand's forwarding `match` with exceptions has the arms that
forward and the arms the enum answers itself, and no wildcard; the only
form that wrote it named every forwarded variant and made the body `_ =>
body`, so a variant left off the list took the body silently. Under `*`, a
body was an error. It is now one `match self` holding the method's own
arms, read by `OwnArms` in `selection.rs` into `Selection::own`;
`select_targets` takes the mark's body and forwards every entry no arm
names, in the builds `OwnArms::leaves`, `check_body` accepts it,
`MethodPlan::takes_own_arms` refuses a `Self` argument and a `by_place`
match, `fallback_for` takes the body for the rest as `rest`, and
`Forwarding::written` puts the arms after the forwarding ones. `WayOut`
gained `own_arms`, so a refusal offers the arm of the variant; the
sentence for an entry without a payload under `*` names its arm, with the
pattern `Unforwardable::rest` spells, where `select_targets`' `takes_arms`,
`MethodPlan::matches_self_alone`, says the method can take one, and keeps
the named list with a body where it cannot; and `standing_alone` in
`marked.rs` closes such a body where the method is kept as written, over
the scrutinee `receiver_scrutinee` gives a boxed or pinned receiver. On a
struct `*` is one field and a body is reported as before. The golden
`own_arms_under_star`, the behaviour test `own_arms_under_star` and the
compile-fail cases `own_arm_naming_no_variant`,
`own_arms_leaving_a_variant_without_a_payload`,
`own_arms_beside_a_self_argument`, `own_arms_over_variants_all_under_cfg`,
`own_arms_over_a_variant_the_method_refuses`,
`own_arms_over_a_variant_that_cannot_be_rebuilt`,
`own_arms_beside_a_self_return_the_lift_cannot_wrap`,
`inherent_all_with_body_on_a_struct`,
`inherent_list_of_every_variant_with_body`,
`star_over_variants_all_under_cfg_without_a_payload` and
`impl_attribute_error_keeps_own_arms_under_a_boxed_receiver` are new; the
snapshots `inherent_all_with_body`, `inherent_all_with_unforwardable` and
`rejected_variant_keeps_enum` moved with the reworded sentences, and
`star_over_variants_without_a_payload_beside_a_self_argument` did not,
pinning the named list kept beside a `Self` argument. No build changes meaning: `*` with a body was an
error, and a body under a named list is unchanged.

### Stage 28: a last field is matched without a rest pattern

Status: landed on 2026-11-16, unreleased, on the branch `arms-under-star`.
`Target::pattern` matched an unnamed marked field that is the last of its
variant or struct as `Last(_, inner, ..)` and `Labelled(_, inner, ..)`, a
rest pattern with nothing to skip, as stage 26 found for a sole field; the
conformance matrix's shape comparison, once it saw a rest pattern, found
it. `PayloadField::Beside` is now `Beside { member, followed }`, built by
`PayloadField::beside(member, count)`, and the pattern writes the rest only
where `followed`: `Last(_, inner)` and `Labelled(_, inner)`, while
`Tagged(inner, ..)`, `Middle(_, inner, ..)` and a named field beside others
keep their `..`. Golden `marked_last_field` is new, and the unit tests
`a_rest_pattern_follows_a_payload_only_where_fields_follow_it` and
`a_target_is_matched_and_built_as_its_holder_and_field_say` in `subject.rs`
pin the patterns. No existing golden and no snapshot moved, and nothing
changes at run time.

## 5. Invariants every stage keeps

- The generated impl is the one a careful hand writes: a `match`, one fully
  qualified call per arm, `#[inline]`, no conversion unless an option asks
  for one. `benchmarks/scripts/codegen.py` keeps checking this.
- Generated bindings have the hygiene of `Span::mixed_site()`, located at
  the arm's `Site`, which only `Site::at` in `site.rs` makes; the receiver
  uses the trait method's own `self` token; a variant's `#[cfg]` is
  carried onto its arm, a trait item's is decided by which carrier the
  trait's crate compiled.
- Every token the engine writes carries a user span, never the call site:
  a function that writes generated code takes a `Site`, and `spans.rs`
  reads the engine's modules and names a bare `quote!`, and an identifier
  or a lifetime built at the call site.
- Nothing is kept between expansions.
- `implement!` keeps its name and reads every prefix a 0.x carrier wrote,
  `v1` and `v2`; the request stays `v1`; a carrier's last rule reports any
  other request; the facade and the macros crate move as one version.
- A rejected subject is still emitted, with its carrier, so the code around
  it compiles.
- The facade stays `no_std` and dependency-free; the minimum Rust version
  stays 1.88.
- Every change to a golden or a `.stderr` file is listed in the stage's
  status line.
- Every sentence the macros write has a `tests/ui` snapshot: `limits.rs`
  reads the sources and the snapshots and names the ones that do not.
- The conformance matrix agrees: a change that moves a cell's standing
  lists the lines of `conformance/known.tsv` it adds or deletes in the
  stage's status line, and a line is deleted by the fix it names.

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
- Settled on 2026-11-12: the `syn` is `syn` 3, with `full`, `visit` and
  `visit-mut`, and no expansion changed with it
  ([the record](../docs/decisions/the-macro-crate-parses-with-syn-3.md)).
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
