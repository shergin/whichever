# Compared with the alternatives

Several crates write the forwarding impl of a trait for an enum over its
implementors: the `match` you would otherwise write by hand for every
method. This page shows where each of them stops, and what `whichever`
does differently.

## `enum_dispatch`

`enum_dispatch` came first, and its mechanism is also its limit. It keeps
the trait's definition in a process-global cache inside the proc-macro
server, and remembers it from one expansion to the next. That has three
consequences:

- **An enum in another crate silently gets no impl.** The cache is empty
  when the enum's crate compiles. So the enum compiles without the trait,
  and the first call fails, far from the cause.
- **Real-world traits panic or are refused.** Associated types,
  associated consts, methods without a receiver, `&Self` arguments and
  lifetime-generic traits are out of reach.
- **Errors lose their spans.** Tokens replayed from a cache no longer
  point at your code.

Its tracker, read on 2026-11-13, still has these open:

- [associated types][ed-50], the most upvoted open issue and open since
  2021;
- [the trait and the enum in different crates][ed-83];
- [`Option<Self>` and `Result<Self, E>` returns][ed-82];
- [traits of other crates][ed-79], and [`Display`][ed-55] in particular;
- [a generic trait on an enum that is not generic][ed-67];
- [IDE support][ed-52].

All of them work in `whichever`.

[ed-50]: https://gitlab.com/antonok/enum_dispatch/-/issues/50
[ed-52]: https://gitlab.com/antonok/enum_dispatch/-/issues/52
[ed-55]: https://gitlab.com/antonok/enum_dispatch/-/issues/55
[ed-67]: https://gitlab.com/antonok/enum_dispatch/-/issues/67
[ed-79]: https://gitlab.com/antonok/enum_dispatch/-/issues/79
[ed-82]: https://gitlab.com/antonok/enum_dispatch/-/issues/82
[ed-83]: https://gitlab.com/antonok/enum_dispatch/-/issues/83

Here is a trait `enum_dispatch` cannot take, dispatched with the same two
attributes:

```rust
#[whichever::dispatchable]
pub trait Storage: Sized {
    type Error;

    fn load(&self, key: &str) -> Result<Vec<u8>, Self::Error>;

    /// Two different backends never share a file.
    #[dispatch(mismatch = false)]
    fn shares_file_with(&self, other: &Self) -> bool;

    fn snapshot(&self) -> Option<Self>;
}

#[derive(Debug)]
#[whichever::dispatch(Storage, Error = StorageError)]
#[whichever::dispatch(whichever::foreign::Display)]
pub enum AnyStorage {
    Memory(MemoryStorage), // Error = Infallible
    Disk(DiskStorage),     // Error = io::Error
}
```

What to notice:

- The variants' own error types meet in one `StorageError` through
  `Into`.
- `shares_file_with` on two different variants returns the `false` the
  trait declares.
- `snapshot` wraps the payload's `Option<Self>` back into its variant.
- `Display` is a trait of `std`, dispatched from `whichever::foreign`.

The whole program is [`examples/storage.rs`](../examples/storage.rs). Run
it with `cargo run --example storage`.

## The crates that followed

The crates that followed work across crates, but each one supports a
different slice of what real traits look like:

- **`ambassador`** handles associated types, associated consts and
  generic traits. It has no `Self` in arguments or returns and no `From`
  impls. An enum takes each const from its first variant. When the
  const's type has a `==` that runs at compile time, such as an
  integer's, the other variants are checked to agree, but only where
  the enum's const is used. A `&str` const on an enum does not compile,
  and there is no way to fix a const's value on the enum.
- **`enum_delegate`** handles `-> Self` and unifies associated types. It
  panics at runtime on `&Self` arguments, and has no `async`, no generic
  traits and no generic methods. It also leaves out `#[inline]`, which
  costs an out-of-line call across a crate boundary
  ([measured](../benchmarks/RESULTS.md)).
- **`delegate`** handles almost anything, because you list every
  signature again by hand.
- **`portrait`, `delegation` and `static-dispatch`** cover plain `&self`
  methods, defaults and generic methods.

Whichever you pick, the first trait with a `&Self` argument, or with an
associated const the variants disagree on, sends you back to writing
the `match` by hand.

## What `whichever` does instead

`whichever` keeps nothing between expansions. `#[dispatchable]` exports a
[carrier macro](glossary.md#carrier-macro): a hidden macro that holds the
trait's signatures, re-exported under the trait's own name. `#[dispatch]`
calls it.

So the trait reaches the enum through ordinary name resolution. It works
across crates, in any order, and every span is the one you wrote.

## Feature by feature

Every row where only `whichever` says yes is a trait that, with any other
crate, goes back to a hand-written `match`.

How to read the table: "yes" means the case works; "no" means it does
not; a dash means the case was not probed; anything else says how the
case is handled.

| | whichever | enum_dispatch 0.3 | ambassador 0.5.1 | portrait 0.3 | enum_delegate 0.2 | delegation 0.5 | static-dispatch 0.3 | delegate 0.13 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Trait and enum in different crates | yes | no, silently | yes | yes | yes | yes | yes | yes, signatures re-listed |
| Traits of `std` or a crate you do not own | yes, `Display`, `io::Read` and more shipped; others restated once | no | yes, restated | - | - | - | - | yes, signatures re-listed |
| Default methods forwarded | yes | yes | yes | yes | yes | yes | yes | if listed |
| `Self`, `&Self`, `&mut Self` arguments | yes, with a declared mismatch value (what to return when the two hold different variants) | no | no | no | panics at runtime | no | no | no |
| `-> Self` | yes | yes | no | no | yes | no | no | with `#[into]` |
| `-> Option<Self>`, `-> Result<Self, E>` | yes | no | - | - | - | - | - | - |
| Associated types | yes | no | yes | no | yes | no | no | by hand |
| Differing associated types unified | yes, through `Into` | no | no | no | yes, wrapper enum | no | no | by hand |
| Associated consts | yes, checked to agree or fixed on the enum | no | yes, checked to agree where the enum's const is used, if its `==` is const; not fixed on the enum | no | no | no | no | by hand |
| Methods without a receiver | with a default body, or written by hand next to the forwarded ones; through a struct, forwarded to the field's type | no | no | no | no | no | no | - |
| Some methods written by hand, the rest forwarded | yes | no | no | no | no | no | no | yes, by listing the rest |
| `async fn` | yes | yes | yes | no | no | no | yes | yes |
| Generic trait at a concrete instantiation | yes | no | yes | no | no | yes | no | by hand |
| Const generics on the trait | yes | - | - | - | - | - | - | - |
| Generic methods | yes | yes | yes | yes | no | yes | yes | yes |
| Lifetime-generic trait and enum | yes | no | yes | no | no | yes | no | by hand |
| Several traits on one enum | yes | yes | yes | yes | no | no | yes | yes |
| Payload with a same-named inherent method | safe | safe | safe | safe | safe | safe | safe | calls it, unless `#[through]` |
| `Box<T>`, `Rc<T>`, `&T` payloads | yes, `#[dispatch(deref)]` | no | with an extra block | no | no | no | no | yes |
| A field of the payload, or of a variant with several fields, as the target | yes, a route or a marked field, with a route of its own for a mutable borrow | no | no | no | no | no | no | yes, by hand |
| `-> impl Trait` | `impl Future<Output = T>` on an enum; any, through a struct | no | through a struct | no | no | no | no | no |
| `From<Payload>` impls | yes, opt-out; `TryFrom<Enum>` opt-in, by value, `&` and `&mut`; `From<Enum> for Target` with `into =` | yes, plus a forced `TryInto` | no | no | yes, with `&` and `&mut` `TryInto` | no | no | no |
| Inherent methods forwarded to some variants, with a fallback | yes | no | no | no | no | no | no | by hand |
| Variants without a payload, left to the fallback | yes | no | - | - | - | - | - | - |
| A struct forwarding to one field | yes | no | yes | yes | no | yes | no | yes |
| An enum another crate defines | yes, restated with `remote`; a struct too | no | yes, restated | no | yes, restated | with `unsafe` | no | no |
| `self: Pin<&mut Self>`, `self: Box<Self>` | yes | - | - | - | - | - | - | - |

Probed on 2026-11-01, with the trait in one crate and the enum in
another, on stable Rust, against the versions shown and `ambassador`
0.5. Two rows of the `ambassador` column, associated consts and
`Box<T>` payloads, were probed again on 2026-11-21 against 0.5.1, on
Rust 1.100.0. A dash means the case was not probed for that crate. The
`whichever` column is what `tests/` checks.

### The `ambassador` column in detail

Associated consts:

- An enum takes each const from its first variant, and a struct from
  its field's type.
- On an enum, the other variants are checked to agree with the first.
  The check is an `assert!` inside the enum's const, so a disagreement
  is an error only where your code uses `<Enum as Trait>::NAME`. If
  nothing uses it, the enum compiles with the first variant's value.
- The check needs a `==` that runs at compile time, which integers have
  and `&str` does not. So an enum over a trait with a `&str` const does
  not compile, even when every variant agrees. A struct with one does.
- Nothing fixes a const's value on the enum. The attribute has no key
  for it, a second impl of the trait conflicts with the derived one,
  and `target = "self"` refuses associated consts.

`Box<T>` payloads, probed with a field or payload of type
`Box<dyn Trait>`:

- A struct reaches the box in two ways, each one more block beside the
  struct. Under `#[delegate_to_methods]`, an impl block of methods that
  return `&dyn Trait` and `&mut dyn Trait` names them with `target_ref`
  and `target_mut`. Under `#[delegate_to_remote_methods]`, with
  `target_ref = "deref"` and `target_mut = "deref_mut"`, a block over the
  struct, which implements `Deref`, does the same.
- An enum's variant with a `Box<dyn Trait>` payload works once
  `Box<dyn Trait>` implements the trait. `#[delegate_to_remote_methods]`
  over `Box<dyn Trait>`, with the same keys, writes that impl, but only
  in the trait's crate: in any other crate, Rust's orphan rule refuses
  it.
- Without one of these blocks, a box is not reached: the derive needs
  the payload or field type itself to implement the trait.
- `Rc<T>` and `&T` were not probed.

## For a struct, beside a forwarding crate

When a struct forwards a trait to one of its fields, two tools help,
whichever way you write the impl:

- **The lint `clippy::missing_trait_methods`.** It reports each
  [provided method](glossary.md#provided-method) an impl leaves out, so
  you do not run the trait's default where you meant to forward to the
  field. It works on a hand-written impl, and on one whose methods the
  `delegate!` macro of `delegate` writes, since Clippy counts the
  methods a macro writes inside your impl. So `delegate`'s "if listed"
  becomes "if listed, and Clippy tells you when one is not". The lint is
  off by default; turn it on with
  `#[deny(clippy::missing_trait_methods)]`.
- **`auto_impl`, for a pointer.** Put `#[auto_impl(&, Box, Arc)]` on a
  trait you own, and it writes the blanket impls, such as
  `impl<T: Trait> Trait for Box<T>`, that forward through the pointer.
  `whichever` does not write these. Its `#[dispatch(deref)]` reaches the
  target through a pointer that a variant or a struct's field holds,
  without an impl for the pointer.
