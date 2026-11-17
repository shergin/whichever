# Troubleshooting

You have a compiler error and want to know what it means and what to do.
Find the message below; each entry gives the cause and the fix.

How errors work in this crate:

- When you ask for something `whichever` cannot forward (a
  [limit](limits.md)), you get a one-sentence error on the item at fault,
  usually saying what to do instead.
- Everything else is rustc reporting on the generated impl, exactly as it
  would on the same impl written by hand.
- The rest of the enum still compiles, so you see the error for the
  thing to fix, and no wall of follow-on errors.

In the messages below, `Shape`, `Any`, `Circle` and the like are the
names from the failing example; your error shows your own names.

On this page:

- [At the enum's `#[dispatch(..)]`](#at-the-enums-dispatch)
- [Inside the generated impl](#inside-the-generated-impl)
- [On a variant](#on-a-variant)
- [On a trait method](#on-a-trait-method)
- [Conversions](#conversions)
- [At the trait](#at-the-trait)
- [In `Cargo.toml`](#in-cargotoml)
- [It compiles, but does something else](#it-compiles-but-does-something-else)
- [Still stuck?](#still-stuck)

## At the enum's `#[dispatch(..)]`

### "cannot find macro `Shape` in this scope"

```text
error: cannot find macro `Shape` in this scope
```

**What it means.** `#[dispatch(Shape)]` reads the trait's signatures from
its [carrier macro](glossary.md#carrier-macro), a hidden macro with the
trait's name. rustc cannot find that macro where the enum is, for one of
two reasons:

- The trait is not marked `#[whichever::dispatchable]`. rustc then adds a
  note: ``= note: `Shape` is in scope, but it is a trait, not a macro``.
- The trait is not imported where the enum is. The trait carries the
  macro, so importing one imports both. rustc then usually suggests the
  `use` line.

**The fix.** Put `#[whichever::dispatchable]` on the trait, or import the
trait in the enum's module, as in `use crate::shapes::Shape;`.

If the trait is not yours, see the next two entries.

### "`std::fmt::Display` is not `#[dispatchable]`"

```text
error: `std::fmt::Display` is not `#[dispatchable]`; dispatch `whichever::foreign::Display` if it is one of the traits `whichever` declares, or restate the trait with `#[whichever::dispatchable(remote = std::fmt::Display)]`
```

**What it means.** You wrote a `std::`, `core::` or `alloc::` path in
`#[dispatch]`. Traits from the standard library carry no carrier macro,
so `whichever` tells you directly instead of letting rustc fail to find
one.

**The fix.** Use the restatement that ships in `whichever::foreign`, such
as `#[whichever::dispatch(whichever::foreign::Display)]`, or restate the
trait yourself with `remote`. See [restatements.md](restatements.md).

### "cannot find `Trait` in `other`"

```text
error[E0433]: cannot find `Plain` in `otherlib`
  |                                 ^^^^^ could not find `Plain` in `otherlib`
```

Older Rust versions, such as 1.88, word the first line differently:

```text
error[E0433]: failed to resolve: could not find `Plain` in `otherlib`
```

**What it means.** On `#[dispatch(other::Trait)]`: the trait belongs to
another crate and has no `#[dispatchable]`, so there is no carrier macro
at that path. This is the same problem as "cannot find macro" above, for
a trait of another crate.

**The fix.** Restate the trait with `remote`:
`#[whichever::dispatchable(remote = other::Trait)]`. See
[restatements.md](restatements.md#restate-any-other-trait).

The same words on a [partial impl block](partial-impl-blocks.md) over
another crate's enum, `#[whichever::dispatch] impl Local for other::Enum
{}`, mean one of two things:

- the enum has no `#[whichever::dispatch]`, and only its own crate can
  add it;
- or the enum is another crate's, and you restate it with `remote`
  ([Enums from other crates](restatements.md#enums-from-other-crates)).

### "written by a newer `whichever` than the one dispatching it"

```text
error: this trait's carrier macro was written by a newer `whichever` than the one dispatching it; update `whichever` in this crate
```

```text
error: this trait's carrier macro was written by an older `whichever` than the one dispatching it; update the trait's crate, or build both crates with the same `whichever` minor
```

Releases up to 0.6 say ``unsupported `whichever` protocol version``
instead.

**What it means.** The trait's crate and the enum's crate build with
different minor versions of `whichever`. The carrier macro is written by
the trait crate's `whichever` and read by the enum crate's. Before 1.0,
Cargo treats two minor versions as two different crates, so one graph
can hold both. A newer `whichever` reads every older carrier macro; an
older one cannot read a newer one, and gives the first message. The
second message is the guard for the other direction: a request from a
newer `whichever` that the older carrier macro does not match.

**The fix.** Build both crates with the same minor version of
`whichever`, as the error says. See
[compatibility.md](compatibility.md#versions-across-crates).

## Inside the generated impl

These errors point into the trait's signatures or at the enum, with a
note that they come from the expansion of the trait's macro.

### "cannot find type `Meters` in this scope"

```text
error[E0425]: cannot find type `Meters` in this scope
   = note: this error originates in the macro `Shape` (in Nightly builds, run with -Z macro-backtrace for more info)
```

On Rust 1.88, the minimum supported version, the code is `E0412`.

**What it means.** A type in the trait's signatures is not in scope where
the enum is. The impl is written into the enum's module, so every type
the trait's signatures name must be in scope there, exactly as for a
hand-written impl. This is the most common new error when you move from
`enum_dispatch`, which sometimes wrote the impl next to the trait.

Some details:

- Paths that start with `crate::` are rewritten to the trait's crate, so
  they work anywhere.
- `super::` and `self::` paths are not rewritten. Do not write them in a
  `#[dispatchable]` trait's signatures.
- A private type or alias in a signature cannot be named from another
  module or crate. It limits the trait to its own crate.
- The attribute does not add the imports for you, because a glob import
  written around the impl would shadow your module's own names.

**The fix.** Import the type where the enum is; rustc usually suggests
the `use` line. Or import the whole trait crate with a glob,
`use trait_crate::*;`. Your module's own names still win over a glob.

To keep the glob out of the enum's file, write one empty partial impl
block per trait in a module of its own. Put the glob there, and import
the payload types there by name:

```rust
#[whichever::dispatch]
impl Client for ClientWrapper {}
```

See [Which names must be in scope](partial-impl-blocks.md#which-names-must-be-in-scope).

### "the trait bound `B: Value` is not satisfied"

```text
error[E0277]: the trait bound `B: Value` is not satisfied
```

It points at the variant's payload.

**What it means.** A payload does not implement the trait. Every
[target](glossary.md#target) must implement it, since the arm calls the
trait method on it.

**The fix.** Implement the trait for that payload. If the variant should
not take part, leave it to a fallback in a
[partial impl block](partial-impl-blocks.md), or reach a target that does
implement the trait with a [route](routes.md).

### Two variants disagree on an associated type

```text
error[E0308]: `match` arms have incompatible types
error[E0271]: type mismatch resolving `<Wide as Store>::Item == u32`
```

**What it means.** The enum takes each associated type from its first
variant, and checks that every other variant has the same one (the
[agreement check](glossary.md#agreement-check)). Here one payload says
`type Item = u32;` and another says `type Item = u64;`.

**The fix.** Give the enum its own type with an
[override](options.md#name--type-override-an-associated-type), as in
`#[whichever::dispatch(Store, Item = u64)]`. The forwarded methods
convert each payload's result into it (here, a `u32` into a `u64`).

### Two variants disagree on an associated const

```text
error[E0080]: evaluation panicked: `KIND` of variant `Ordering` differs from the first variant's; set it on the enum with `KIND = value`
```

**What it means.** The same check, for an associated const: one variant
has a different value from the first.

**The fix.** Set the value on the enum, as in
`#[whichever::dispatch(Proof, KIND = "any")]`. See
[options.md](options.md#name--value-override-an-associated-const).

## On a variant

### "holds no payload" or "has named fields, not one payload"

```text
error: `Nothing` holds no payload, so no trait method can be forwarded to it; write it as `Nothing(Payload)`, or forward inherent methods instead, whose body answers for it
error: `A` holds 2 fields, not one payload, so no trait method can be forwarded to it; write it as `A(Payload)`, mark the field to forward to with `#[dispatch]`, or forward inherent methods instead, whose body answers for it
error: `A` has named fields, not one payload, so no trait method can be forwarded to it; write it as `A(Payload)`, mark the field to forward to with `#[dispatch]`, or forward inherent methods instead, whose body answers for it
```

**What it means.** A variant a trait is forwarded to must hold exactly
one unnamed field, `Name(Payload)`, so the arm has one value to call the
method on. A unit variant, or a variant with several fields, has no such
value.

**The fix.** Pick the one that fits:

- write the variant as `Name(Payload)`;
- mark the field to forward to with `#[dispatch]`, as in
  `Node { id: u32, #[dispatch] data: Data }`;
- or forward [inherent methods](inherent-methods.md) instead, where the
  method's body handles such variants.

The error follows the variant's `#[cfg]`, so a test-only mock that is
compiled out is no error. See [limits.md](limits.md).

### "requires `Self: Sized`, and `Boxed` holds a trait object"

```text
error: `doubled` requires `Self: Sized`, and `Boxed` holds a trait object, which is not; leave it to the trait's default with `#[dispatch(default)] fn doubled(..);` in a `#[whichever::dispatch] impl Shape for Any` block, hold `Box<dyn Shape>` without `deref` where it implements the trait itself, or implement the method by hand
```

**What it means.** A `deref` variant holds a trait object, such as
`Box<dyn Shape>`. A method declared `where Self: Sized` cannot be called
on a trait object, so it cannot be forwarded to that variant.

**The fix.** The error names three: the two below, or implementing the
method by hand.

- Leave the method to the trait's default in a
  [partial impl block](partial-impl-blocks.md). The other methods are
  still forwarded.

  ```rust
  #[whichever::dispatch]
  impl Shape for Any {
      #[dispatch(default)]
      fn doubled(&self) -> u32;
  }
  ```

- If the pointer implements the trait itself, hold it as a plain
  payload, without `deref`. `Box<dyn Iterator>` is an `Iterator`, so
  every method is forwarded to the box, with no partial impl block. See
  [`Self: Sized` and boxed iterators](restatements.md#self-sized-and-boxed-iterators).

## On a trait method

### "takes `Self` arguments": a missing `mismatch`

```text
error: `same` takes `Self` arguments; give the result for two different variants with `#[dispatch(mismatch = <expr>)]` on the trait method or `mismatch(same = <expr>)` on the enum
```

**What it means.** A method like `fn same(&self, other: &Self) -> bool`
is forwarded only when `self` and `other` hold the same variant. When
they hold different variants, there is no payload method to call, so
`whichever` needs the value to return.

**The fix.** Give that value, on the trait method or on the enum:

```rust
#[whichever::dispatchable]
pub trait Same {
    #[dispatch(mismatch = false)]
    fn same(&self, other: &Self) -> bool;
}
```

```rust
#[whichever::dispatch(Same, mismatch(same = false))]
pub enum Any {
    A(A),
    B(B),
}
```

See [`mismatch`](options.md#mismatch-what-to-return-when-two-variants-differ).

### "has no receiver, so there is no variant to dispatch on"

```text
error: `presets` has no receiver, so there is no variant to dispatch on; give it a default body, or write it in a `#[whichever::dispatch] impl Factory for Kind` block
```

**What it means.** A method without `self`, such as `fn new() -> Self`,
has no value to `match` on, so no variant to forward to.

**The fix.** Give the method a default body in the trait, which the enum
uses, or write it yourself in a
[partial impl block](partial-impl-blocks.md).

### "a return-position `impl Trait` cannot be dispatched"

```text
error: a return-position `impl Trait` cannot be dispatched by a `match` because every variant returns a different opaque type; return a concrete type or a boxed trait object, or `impl Future<Output = T>`, which is awaited like an `async fn`
```

**What it means.** Each payload's `impl Trait` is a different hidden
type, and a `match` must return one type.

**The fix.** Return a concrete type or a boxed trait object.
`impl Future<Output = T>` is fine: it is awaited like an `async fn`.

### "this receiver type is not supported"

```text
error: this receiver type is not supported; a receiver is `self`, `&self`, `&mut self`, `self: Box<Self>`, `self: Pin<&Self>` or `self: Pin<&mut Self>`
```

**What it means.** `self: Rc<Self>`, `self: Arc<Self>` and
`self: Pin<Box<Self>>` would need a refcount or an allocation to forward,
which hand-written code would not do.

**The fix.** Use one of the receivers the error lists, or implement the
method by hand. See [Receivers](limits.md#receivers).

### Other errors on a trait method

The other one-sentence errors on a method, about `Self` in a return
type, routes, `deref` variants and the arms the enum handles itself, are
each on [limits.md](limits.md), grouped by topic, with the fix.

## Conversions

### "conflicting implementations of trait `From<Circle>`"

```text
error[E0119]: conflicting implementations of trait `From<Circle>` for type `Any`
```

**What it means.** The enum gets `From<Payload>` for each payload type.
You also have a `From` for the same type: written by hand, or supplied by
a derive.

**The fix.** Keep that variant out of the generated `From` with
`#[dispatch(from = false)]`:

```rust
#[whichever::dispatch(Shape)]
enum Any {
    #[dispatch(from = false)]
    Circle(Circle),
    Square(Square),
}

impl From<Circle> for Any {
    fn from(circle: Circle) -> Self {
        Any::Circle(circle)
    }
}
```

Or turn all of them off with `from = false` on the enum. See
[options.md](options.md#from--false-no-from-impls).

### "the trait bound `Any: From<Circle>` is not satisfied"

```text
error[E0277]: the trait bound `Any: From<Circle>` is not satisfied
```

**What it means.** Two variants hold the same payload type, as in
`Small(Circle)` and `Large(Circle)`. A `From<Circle>` could not know
which variant to build, so that payload type gets no `From` impl at all.

**The fix.** Build the variant you mean by name: `Any::Large(circle)`.

### A payload that is a type parameter: "`From<I>` would overlap"

```text
error: `I` is a type parameter of `Adapter`, so `From<I>` would overlap the other variants' `From` impls; add `from = false` on the enum, or `#[dispatch(from = false)]` on this variant
```

**What it means.** A payload that is a type parameter would get
`From<T>`, which overlaps the `From` impls of the other variants.

**The fix.** As the error says: `from = false` on the enum, or
`#[dispatch(from = false)]` on that variant. See
[Payloads that are type parameters](options.md#payloads-that-are-type-parameters).

### "`Any` doesn't implement `Debug`" on `.unwrap()`

```text
error[E0277]: `Any` doesn't implement `Debug`
```

**What it means.** With `try_from = true`, `Circle::try_from(any)`
returns the enum itself as the error, so a failed conversion hands the
value back.
`.unwrap()` prints the error when it panics, so it needs the enum to be
`Debug`.

**The fix.** Derive `Debug` on the enum (and its payloads), or match on
the result instead of unwrapping it:

```rust
#[derive(Debug)]
#[whichever::dispatch(Shape, try_from = true)]
enum Any {
    Circle(Circle),
    Square(Square),
}
```

## At the trait

### "`#[async_trait]` rewrites the signatures after `whichever` has read them"

```text
error: `#[async_trait]` rewrites the signatures after `whichever` has read them; put `#[whichever::dispatchable]` below it
```

**What it means.** Attributes run from the top down. With
`#[dispatchable]` on top, `whichever` reads `async fn` signatures that
`#[async_trait]` then rewrites, so the carrier macro would not match the
trait.

**The fix.** Put `#[async_trait]` first and `#[whichever::dispatchable]`
below it.

### "`#[typetag::serde]` has added its per-impl methods to the trait"

```text
error: `#[typetag::serde]` has added its per-impl methods to the trait, which a forwarder would answer with the payload's tag; put `#[whichever::dispatchable]` above it, and `#[typetag::serde(name = "..")]` on the enum's partial impl block
```

**What it means.** typetag is the other way round. With it on top, it
has already added its hidden methods when `whichever` reads the trait.
Forwarding them would serialise the enum under its payload's tag.

**The fix.** Put `#[whichever::dispatchable]` above `#[typetag::serde]`,
and `#[typetag::serde(name = "..")]` on the enum's
[partial impl block](partial-impl-blocks.md).

### "the name `__whichever_Shape_...` is defined multiple times"

```text
error[E0428]: the name `__whichever_Shape_1f42557c6b2393d4_3_9` is defined multiple times
```

**What it means.** One `macro_rules!` emits the same `pub` trait,
written out in full in its body, in two places. The carrier macro's name
comes from the trait's tokens and their positions, so both copies get
the same name. A `pub` trait's carrier macro is exported at the crate
root, where the two collide.

A macro whose invocation supplies at least one token of the trait, its
name or a method's name, does not have this problem.

**The fix.** Give each invocation its own name with
`#[dispatchable(name = ..)]`:

```rust
macro_rules! shape_trait {
    ($name:ident) => {
        #[whichever::dispatchable(name = $name)]
        pub trait Shape {
            fn area(&self) -> f64;
        }
    };
}

pub mod a {
    shape_trait!(shape_a);
}

pub mod b {
    shape_trait!(shape_b);
}
```

### "ambiguous import visibility: pub(crate) or pub"

```text
error: ambiguous import visibility: pub(crate) or pub
   = note: `#[deny(ambiguous_import_visibilities)]` implied by `#[deny(future_incompatible)]`
```

Without `#![deny(future_incompatible)]` this is a warning.

**What it means.** A private glob import brings the trait's own name back
into the trait's module. The typical case is `use crate::prelude::*;` at
a crate root whose prelude re-exports a trait declared there. The
carrier macro is re-exported under the trait's name, so the glob import
and that re-export both bring a macro of that name into the module, and
rustc reports the conflict (lint `ambiguous_import_visibilities`).

**The fix.** Declare the trait in a module of its own and re-export it,
or import the prelude's items by name in that module instead of with a
glob.

### Unused imports next to a restatement

```text
warning: unused imports: `Formatter` and `Result`
```

**What it means.** Only the carrier macro reads a
[restatement](glossary.md#restatement); nothing else is emitted for it.
So the imports its signatures use count as unused where the restatement
is, while the enum's module still needs them.

**The fix.** Write the signature types in full in the restatement, such
as `core::fmt::Formatter<'_>`, as `whichever::foreign` does. Or import
them where the enum is, not where the restatement is. See
[restatements.md](restatements.md).

## In `Cargo.toml`

### "cannot find `whichever` in the list of imported crates"

```text
error[E0433]: cannot find `whichever` in the list of imported crates
```

Older Rust versions, such as 1.88, word it differently:

```text
error[E0433]: failed to resolve: could not find `whichever` in the list of imported crates
```

Errors about missing trait methods on the enum follow it.

**What it means.** You renamed the dependency, as in
`wc = { package = "whichever", .. }`, and turned off the default
feature `renamed-dependency`. That feature is what looks the new name up.

**The fix.** Keep the default features on. See
[compatibility.md](compatibility.md#dependencies-and-the-renamed-dependency-feature).

### A dependency cycle through `proc-macro-crate`

**What it means.** The `renamed-dependency` feature depends on
`proc-macro-crate`. A crate that `proc-macro-crate` itself depends on,
such as `toml_edit`, would then depend on itself, and Cargo rejects the
cycle.

**The fix.** That crate takes `whichever` with
`default-features = false`, and does not rename it.

## It compiles, but does something else

### A provided method does not run the payload's override

No error here. The enum compiles, but a provided method (a method with a
default body) returns the trait's default, not what the payload's own
version returns.

**What it means.** The trait is a [restatement](restatements.md), and
the restatement does not list that method. A restatement forwards
exactly the methods it lists. A method left out falls back to the
trait's default on the enum, whatever the payload would do.

The common case is tokio's `AsyncWrite`. Its streams return `true` from
`is_write_vectored`, but a restatement of the three required methods
alone makes the enum return `false`. A caller such as hyper then copies
every buffer into one.

**The fix.** List the provided methods a payload overrides in the
restatement. See
[List the provided methods a payload overrides](restatements.md#list-the-provided-methods-a-payload-overrides).

## Still stuck?

Look at what was generated. `cargo expand` (from the `cargo-expand`
crate) prints your code with the macros expanded, so you can read the
impl that rustc is complaining about:

```bash
cargo expand --lib path::to::module
```

[what-is-generated.md](what-is-generated.md) explains what to expect in
it.

If the error still makes no sense, or `whichever` gives an error for code
you think it should accept, open an issue at
<https://github.com/shergin/whichever/issues>. A small example that
reproduces the error helps most.
