# whichever

[![crates.io](https://img.shields.io/crates/v/whichever.svg)](https://crates.io/crates/whichever)
[![docs.rs](https://docs.rs/whichever/badge.svg)](https://docs.rs/whichever)
[![CI](https://github.com/shergin/whichever/actions/workflows/ci.yml/badge.svg)](https://github.com/shergin/whichever/actions/workflows/ci.yml)

Implements a trait for an enum by forwarding to whichever variant it holds.

Put one attribute on a trait and one on an enum, and the enum implements
the trait. Each method becomes a `match` that calls the value inside the
variant. You get the speed of an enum, with no `Box`, no vtable and no
allocation, and you write none of the boilerplate.

- **Zero cost.** The generated code is the `match` you would write by hand,
  down to the machine instructions.
- **Works across crates.** The trait can live in one crate and the enum in
  another.
- **Handles real traits.** Associated types and consts, `Self` in arguments
  and return types, generic traits, `async fn`, `Box` and `Pin` receivers.
- **Works with traits you don't own.** `Display`, `Error`, `Iterator`,
  `io::Read`, `io::Write` and more are ready to use.
- **Lets you write the exceptions.** Implement a method by hand, and the
  rest are still forwarded.
- **Clear errors.** When something can't be dispatched, you get one
  sentence pointing at the exact line.
- **`no_std`**, with no dependencies beyond its own macros.

Coming from `enum_dispatch`? The attributes will feel familiar, and
the traits it can't handle work here. See [how it compares](#why-another-one).

```rust
// In the crate that owns the trait.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
}

// In any crate that owns an enum over implementors.
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

`AnyShape` now implements `Shape` with one `match` per method, each arm a
fully qualified call into the variant's payload, and gets `From<Circle>` and
`From<Square>`:

```rust
impl Shape for AnyShape {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            AnyShape::Circle(inner) => <Circle as Shape>::area(inner),
            AnyShape::Square(inner) => <Square as Shape>::area(inner),
        }
    }
    #[inline]
    fn name(&self) -> &'static str {
        match self {
            AnyShape::Circle(inner) => <Circle as Shape>::name(inner),
            AnyShape::Square(inner) => <Square as Shape>::name(inner),
        }
    }
}
```

That is the code a careful hand would write: no trait object, no closure,
no conversion, and a variant's override of a default method is never hidden
by the default.

## Why another one

An enum over the implementors of a trait is the cheapest way to hold "one
of these": no allocation, no vtable, and every arm visible to the
optimiser. The forwarding impl is the price, paid again for every method
and every variant, and every crate in this family exists to pay it for you.

`enum_dispatch` came first, and its mechanism is its limit. It keeps the
trait's definition in a process-global cache inside the proc-macro server,
and remembers it from one expansion to the next. That is why:

- **An enum in another crate gets no impl, silently.** The cache is empty
  when the enum's crate compiles, so the enum compiles without the trait and
  the first call fails, far from the cause.
- **Real trait shapes panic or are refused.** Associated types, associated
  consts, methods without a receiver, `&Self` arguments and lifetime-generic
  traits are out of reach.
- **Errors lose their spans.** Tokens replayed from a cache no longer point
  at your code.

The crates that followed work across crates, but each supports a different
slice of what real traits look like:

- `ambassador` handles associated types and generic traits, but no `Self`
  in arguments or returns, no associated consts and no `From` impls.
- `enum_delegate` handles `-> Self` and unifies associated types, but
  panics at runtime on `&Self` arguments, has no `async`, generic traits or
  generic methods, and leaves out `#[inline]`, which costs from 15% to 4x
  across a crate boundary.
- `delegate` handles almost anything, because you list every signature
  again by hand.
- `portrait`, `delegation` and `static-dispatch` cover plain `&self`
  methods, defaults and generic methods.

Whichever you pick, the first trait with a `&Self` argument or an
associated const sends you back to writing the `match` by hand.

`whichever` keeps nothing between expansions. `#[dispatchable]` exports a
carrier macro holding the trait's tokens, re-exported under the trait's own
name, and `#[dispatch]` calls it. The trait reaches the enum through
ordinary name resolution, so it works across crates, in any order, and
every span is the one you wrote.

On top of that mechanism, `whichever` covers the shapes real traits have,
several of them in no other crate of the family:

- **Associated types and consts**, taken from the first variant and checked
  to agree on every other, or fixed on the enum. Variants with their own
  error types share one enum through `Into`:
  `#[dispatch(Storage, Error = StorageError)]`.
- **`Self` in signatures**: `&Self` and `Self` arguments with a declared
  fallback for mismatched variants, and `-> Self`, `-> Option<Self>`,
  `-> Result<Self, E>` wrapped back into the variant.
- **Traits you do not own.** `Display`, `Debug`, `Error`, `PartialEq`,
  `Iterator`, `Read`, `Write`, `BufRead` and `Seek` ship ready to dispatch;
  any other trait is restated once.
- **Some methods by hand, the rest forwarded**: constructors, methods
  without a receiver, or a method the enum answers differently from every
  payload.
- **Generic traits** at a concrete instantiation, with type, lifetime and
  const parameters.
- **`Box<T>`, `Rc<T>` and `&T` payloads**, dispatched like a plain `T`.
- **`async fn`, `impl Future`, `self: Pin<&mut Self>` and
  `self: Box<Self>`.**
- **Inherent methods** forwarded to some variants, with a fallback for the
  rest.
- **A struct forwarding to one field**, for the public wrapper around a
  private dispatch enum.

Every unsupported shape is still a clear error: one sentence on the trait
item or variant at fault, and the rest of the enum compiles around it.
Every capability above has a test in `tests/`, the generated impls are
pinned by expansion goldens, and CI builds the trait and the enum in
separate crates.

## Moving from `enum_dispatch`

The swap is mechanical:

| `enum_dispatch` | `whichever` |
| --- | --- |
| `#[enum_dispatch]` on the trait | `#[whichever::dispatchable]` |
| `#[enum_dispatch(Trait)]` on the enum | `#[whichever::dispatch(Trait)]` |
| `#[enum_dispatch(A, B)]` | `#[whichever::dispatch(A)]` and `#[whichever::dispatch(B)]` |
| `#[enum_dispatch(Enum)]` on the trait | name the trait on the enum instead |
| `enum E { Circle, Square }` | `enum E { Circle(Circle), Square(Square) }` |
| `From<Payload>` | the same, opt out with `from = false` |
| `TryInto<Payload>`, error `&'static str` | `try_from = true`: `TryFrom<E> for Payload`, the error hands the enum back |

Three things to watch:

- The types in the trait's signatures must be in scope where the enum is,
  as for a hand-written impl. `enum_dispatch` sometimes emitted the impl
  next to the trait instead, so expect a few new `use` lines at the enum,
  and a few unused ones at the trait.
- With `#[async_trait]`, put `#[whichever::dispatchable]` below it; with
  `#[typetag::serde]`, above it. Either order the other way round is
  reported.
- A generic trait names its parameters: `#[whichever::dispatch(Visitor<'a>)]`.

## Performance

`whichever` costs nothing over the hand-written `match`.
[`benchmarks/`](benchmarks/README.md) compares it with a hand-written impl,
with `Box<dyn Trait>` and with the enums of `enum_dispatch`, `ambassador`,
`enum_delegate`, `delegation` and `delegate`, calling from a crate other
than the enum's, without LTO, over collections of 2 to 32 variants in
uniform, cyclic and random layouts. From the last run, on an Apple M1 Pro
([full tables](benchmarks/RESULTS.md)):

- `whichever` performs the same as the hand-written `match`: the functions
  it generates are identical to the hand-written ones, instruction for
  instruction.
- Over a random 8-variant collection, one `&self` call costs 1.7 ns per
  element against 2.3 ns through a `Box<dyn Trait>` when the method bodies
  can be inlined, and 2.1 ns against 2.3 ns when they cannot. Most of the
  enum's advantage is what the optimiser gets to see, not the dispatch.
- Where the payload's place matters, the gap is wide: 1.7 ns against 8.1 ns
  per element over 4 Mi scattered elements, and a collection is built 13x
  faster without a box per element.
- A generated impl without `#[inline]` (`enum_delegate`, `delegation`) pays
  an extra out-of-line call across the crate boundary, from 15% to 4x
  depending on the method and the variant count.

Whether an enum is the right choice for a given program, and how to get
these numbers out of it, is the subject of
[docs/when-to-use-enum-dispatch.md](docs/when-to-use-enum-dispatch.md).

## Compared with the alternatives

Every row where only `whichever` says yes is a trait that, with any other
crate, goes back to a hand-written `match`.

| | whichever | enum_dispatch 0.3 | ambassador 0.5 | portrait 0.3 | enum_delegate 0.2 | delegation 0.5 | static-dispatch 0.3 | delegate 0.13 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Trait and enum in different crates | yes | no, silently | yes | yes | yes | yes | yes | yes, signatures re-listed |
| Traits of `std` or a crate you do not own | yes, `Display`, `io::Read` and more shipped; others restated once | no | yes, restated | - | - | - | - | yes, signatures re-listed |
| Default methods forwarded | yes | yes | yes | yes | yes | yes | yes | if listed |
| `Self`, `&Self`, `&mut Self` arguments | yes, with a declared fallback | no | no | no | panics at runtime | no | no | no |
| `-> Self` | yes | yes | no | no | yes | no | no | with `#[into]` |
| `-> Option<Self>`, `-> Result<Self, E>` | yes | no | - | - | - | - | - | - |
| Associated types | yes | no | yes | no | yes | no | no | by hand |
| Differing associated types unified | yes, through `Into` | no | no | no | yes, wrapper enum | no | no | by hand |
| Associated consts | yes, checked to agree or fixed on the enum | no | no | no | no | no | no | by hand |
| Methods without a receiver | with a default body, or written by hand next to the forwarded ones | no | no | no | no | no | no | - |
| Some methods written by hand, the rest forwarded | yes | no | no | no | no | no | no | yes, by listing the rest |
| `async fn` | yes | yes | yes | no | no | no | yes | yes |
| Generic trait at a concrete instantiation | yes | no | yes | no | no | yes | no | by hand |
| Const generics on the trait | yes | - | - | - | - | - | - | - |
| Generic methods | yes | yes | yes | yes | no | yes | yes | yes |
| Lifetime-generic trait and enum | yes | no | yes | no | no | yes | no | by hand |
| Several traits on one enum | yes | yes | yes | yes | no | no | yes | yes |
| Payload with a same-named inherent method | safe | safe | safe | safe | safe | safe | safe | calls it, unless `#[through]` |
| `Box<T>`, `Rc<T>`, `&T` payloads | yes, `#[dispatch(deref)]` | no | no | no | no | no | no | yes |
| `-> impl Trait` | `impl Future<Output = T>` on an enum; any, through a struct | no | through a struct | no | no | no | no | no |
| `From<Payload>` impls | yes, opt-out; `TryFrom<Enum>` opt-in | yes, plus a forced `TryInto` | no | no | yes | no | no | no |
| Inherent methods forwarded to some variants, with a fallback | yes | no | no | no | no | no | no | by hand |
| Variants without a payload, left to the fallback | yes | no | - | - | - | - | - | - |
| A struct forwarding to one field | yes | no | yes | yes | no | yes | no | yes |
| `self: Pin<&mut Self>`, `self: Box<Self>` | yes | - | - | - | - | - | - | - |

Probed on 2026-11-01 against the versions shown, with the trait in one
crate and the enum in another, on stable Rust. A dash means the case was
not probed for that crate. The `whichever` column is what `tests/` checks.

## Documentation

- This file: what the attributes generate, every option, and the limits.
  The API reference is on [docs.rs](https://docs.rs/whichever).
- [docs/](docs/README.md): what to read when; the vision with its five
  rules, and a decision record per choice made among real alternatives.
- [When enum dispatch pays](docs/when-to-use-enum-dispatch.md): choosing
  between an enum, a trait object and a generic, with the measurements
  behind each piece of advice.
- [Benchmarks](benchmarks/README.md): what is compared with what, how, and
  how to run it; [results](benchmarks/RESULTS.md) from the last run.

## Options

All options follow the trait path in `#[dispatch(..)]`.

- `Name = Type` fixes the associated type `Name` instead of taking it from
  the first variant. A method returning `Self::Name` is converted with
  `Into`, one returning `Result<_, Self::Name>` with `map_err(Into::into)`,
  and `Option<Self::Name>`, `Result<Self::Name, E>` and `Result<Self,
  Self::Name>` are mapped the same way, so variants with their own error
  types can share one enum:

  ```rust
  #[whichever::dispatch(Storage, Error = StorageError)]
  pub enum AnyStorage {
      Memory(MemoryStorage), // Error = StorageError
      Disk(DiskStorage),     // Error = io::Error, which is Into<StorageError>
  }
  ```

- `Name = value` fixes the associated const `Name`, for a const that
  differs per payload by design, such as a kind discriminator:

  ```rust
  #[whichever::dispatch(Proof, KIND = "any")]
  pub enum AnyProof {
      Encoding(EncodingProof), // KIND = "encoding"
      Ordering(OrderingProof), // KIND = "ordering"
  }
  ```

- `mismatch(method = expr, ..)` gives the result of `method` when its
  `Self` arguments hold a different variant than `self`. The trait author
  can decide once instead, with `#[dispatch(mismatch = expr)]` on the
  method:

  ```rust
  #[whichever::dispatchable]
  pub trait Shape {
      /// Two different kinds of shape are never congruent.
      #[dispatch(mismatch = false)]
      fn congruent(&self, other: &Self) -> bool;
  }
  ```

  An expression on the trait method may use the method's parameters, a
  `Self` argument included; one on the enum can use neither them nor
  `self`, since the trait's tokens reach the enum through a `macro_rules!`
  body and carry its hygiene.

- `from = false` suppresses the `From<Payload>` impls. A payload type held
  by two variants gets none either way.
- `try_from = true` adds `TryFrom<Enum> for Payload` for every payload type,
  with the enum as the error, so a mismatch hands the value back; `.unwrap()`
  on the result therefore needs the enum to be `Debug`.

Three options go on a variant. `#[dispatch(from = false)]` leaves the
variant out of the `From` and `TryFrom` impls, for a payload such as
`String` that should not convert into the enum, and
`#[dispatch(try_from = true)]` adds the `TryFrom` impl for that payload
alone. `#[dispatch(deref)]` calls the trait on what the payload
dereferences to, so `Box<T>`, `Rc<T>`, `Arc<T>`, `&T` and `&mut T`
payloads dispatch like a plain `T`:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape<'a> {
    #[dispatch(deref)]
    Boxed(Box<Circle>),
    #[dispatch(deref)]
    Borrowed(&'a Square),
    Plain(Square),
}
```

Several traits are dispatched by repeating the attribute; a supertrait is
one more attribute:

```rust
#[whichever::dispatch(Shape)]
#[whichever::dispatch(Drawable)]
pub enum AnyShape { /* .. */ }
```

A generic trait is dispatched at an instantiation named on the enum: a
concrete one, `#[whichever::dispatch(Encoder<Json>)]`, or the enum's own
parameters, `#[whichever::dispatch(GridTrait<T>)]` on `enum Grid<T>`. Type,
lifetime and const parameters are substituted in every signature, and a
projection `F::Value` becomes `<Json as Format>::Value` when `F` has one
bound; with several bounds the trait has to spell `<F as Format>::Value`
itself.

## Inherent methods

An enum often needs methods of its own that only some variants can answer.
`#[dispatch]` on an inherent `impl` block forwards each marked method to the
named variants' method of the same name; the body is what every other
variant returns. `*` names every variant, and the method then has no body:

```rust
#[whichever::dispatch]
impl AnyShape {
    /// Only circles have a radius.
    #[dispatch(Circle, with = Some)]
    pub fn radius(&self) -> Option<f64> {
        None
    }

    #[dispatch(*)]
    pub fn describe(&self) -> String;

    #[dispatch(*, call = describe)]
    pub fn label(&self) -> String;
}
```

Each arm calls the payload's method by method-call syntax,
`inner.describe()`, so autoref, autoderef and inherent-before-trait
resolution apply as in hand-written code. `with = path` wraps each forwarded
result in a call to `path`, which may be a generic function, called
separately in each arm, so payloads whose results differ can meet in one
declared return type; `call = name` forwards to a method of another name;
`async` methods and methods returning `impl Future` are awaited; and
methods without the attribute are left alone. A method returning `Self`,
`Option<Self>` or `Result<Self, E>` wraps the payload's result back into
the variant, as trait dispatch does. With `with`, the payload returns the
plain value and `with` lifts the wrapped variant: `#[dispatch(Circle, with =
Some)]` on `fn grown(&self) -> Option<Self>` calls a `Circle::grown` that
returns a `Circle`. A `Self` argument is the enum itself and is handed to
the payload's method as it is. A variant under `#[cfg]` may be named; where
it is compiled out, the method takes its body. The `impl` block has to be
in the enum's crate, which inherent impls are anyway; a bare `#[dispatch]`
on an enum registers it for this without dispatching a trait. A block on a
dispatch struct forwards the same way, under `*` or the field's name; there
a `Self` argument is the other value's field, as in trait dispatch, and a
return-position `impl Trait` is forwarded as it is.

## Traits from `std` and other crates

`#[dispatch]` reads a trait's signatures from the carrier macro that
`#[dispatchable]` leaves next to the trait, so a trait from `std` or from a
crate you do not own cannot be dispatched as it is. Restate it once, with
`remote` naming the real trait, and dispatch the restatement:

```rust
#[whichever::dispatchable(remote = std::io::Write)]
trait Write {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    fn flush(&mut self) -> std::io::Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
}

#[whichever::dispatch(Write)] // implements std::io::Write
pub enum Stream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(UnixStream),
}
```

Nothing is emitted for the restatement but the macro, so the name is free
for `use std::io::Write;` in the same module. Exactly the methods restated
are forwarded, provided ones included; a changed signature fails to compile
against the real trait. Write the signature types in full, as
`whichever::foreign` does, or import them where the enum is: nothing but
the macro reads the restatement, so its own imports count as unused. Bounds
on a restated associated type may be left out. A restatement of an
`#[async_trait]` trait carries `#[async_trait]` above
`#[dispatchable(remote = ..)]`, as the trait itself does. The restatements an enum most often needs ship in
[`whichever::foreign`](https://docs.rs/whichever/latest/whichever/foreign/):
`Display`, `Debug`, `Error`, `PartialEq`, `Iterator`, `Read`, `Write`,
`BufRead` and `Seek`, each forwarding every provided method a `match` can
forward. `#[dispatch(whichever::foreign::Display)]` on an error enum
replaces the usual hand-written `match`, and the variant that holds a
`String` keeps `From` away with `#[dispatch(from = false)]`:

```rust
#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
pub enum ClientError {
    Io(io::Error),
    Parse(ParseIntError),
    #[dispatch(from = false)]
    BadUrl(String),
}
```

`whichever::foreign::Error` forwards `source` the same way when every
payload is an error type. When one is not, as the `String` above, or when
the enum answers a method differently from every payload, that method goes
in a [partial impl block](#methods-the-enum-implements-itself) over the
declaration: `#[dispatch] impl whichever::foreign::Error for ClientError
{ fn source(&self) -> .. { None } }`.

## Methods the enum implements itself

A trait method without a receiver, such as a constructor, has no variant to
dispatch on, and sometimes the enum's answer to a method differs from every
payload's. Put `#[dispatch]` on a trait impl block holding just those
items, and the rest of the trait is forwarded around them:

```rust
#[whichever::dispatch]
pub enum Handler {
    Text(TextHandler),
    Map(MapHandler),
}

#[whichever::dispatch]
impl HandlerTrait for Handler {
    fn from_handler(handler: Handler) -> Option<Self> {
        Some(handler)
    }
}
```

The enum then does not name that trait in its own attribute, since the
block is the impl; naming it in both is reported on the block. Options for
the trait, `Name = Type` and `mismatch`, go on the block's attribute; an
associated type written in the block counts as an override too. A generic
associated type is declared here, since no first variant can supply one,
`type Item<'a> = AnyItem<'a> where Self: 'a;`, and each payload's own
converts into it through `Into`, as for any override. A method written in
the block replaces the forwarding for every variant, provided methods
included, which is the way to leave a payload's method out on purpose; one
written under `#[cfg]` replaces it where the `cfg` holds and is forwarded
where it does not, as a hand writes the two arms. The block's own header,
generics and bounds included, becomes the impl's header, and may name the enum's parameters as it likes, `impl<T, Ty> Get<T>
for ImmOrNode<Ty>`. The arms name the payload types as the enum spells
them, so a block in another module needs them in scope there. Outer
attributes on the block are carried onto the impl: `#[typetag::serde(name =
"..")]` goes there, with `#[whichever::dispatchable]` above
`#[typetag::serde]` on the trait, so that typetag's own methods are the
block's and never forwarded. Like inherent blocks, it has to be in the
enum's crate.

## Forwarding from a struct field

A dispatch enum is often private, behind a public struct that forwards the
same trait by hand, and the hand-written half forwards only the required
methods, so a payload's override of a provided method never reaches the
public type. `#[dispatch]` on the struct forwards to its one field, or to
the field marked `#[dispatch]`, every method included:

```rust
#[whichever::dispatch(FlowCtrlHooks)]
enum FlowCtrlInner {
    Window(WindowFlowCtrl),
    XonXoff(XonXoffFlowCtrl),
}

#[whichever::dispatch(FlowCtrlHooks)]
pub struct StreamFlowCtrl {
    #[dispatch]
    inner: FlowCtrlInner,
    hooks: Hooks,
}
```

The field may be any implementor, not only a dispatch enum, so the same
attribute replaces a field-delegation derive:

```rust
#[whichever::dispatch(Addresses)]
pub struct Beacon {
    #[dispatch]
    header: ManagementHeader,
    body: Vec<u8>,
}
```

`#[dispatch(deref)]` on the field reaches through a `Box` or another
pointer. A method returning `Self` is rebuilt only around a struct with no
other field; otherwise it is an error, and the method goes in a partial impl
block, which works on a struct like on an enum. A struct's `match` has one
arm, so it forwards a return-position `impl Trait` as it is, which an enum
cannot. A struct gets no `From` impls. `Self` arguments need no `mismatch`:
two values of the struct always hold the same field, and the argument is
unwrapped to it.

## What each trait item becomes

| Trait item | Enum impl |
| --- | --- |
| `type X;` | `type X = <FirstVariant as Trait>::X;`, or the `X = Type` option. Every other variant is checked to agree; a disagreement is reported on that variant. |
| `const N: T;` | `const N: T = <FirstVariant as Trait>::N;`, or the `N = value` option. For an integer, `bool`, `char` or `&str` const on a non-generic enum, every other variant is checked to agree at compile time, with the error on that variant. |
| method with a receiver | A `match` over the variants calling `<Payload as Trait>::method(payload, args..)`, with `#[inline]` or the trait method's own `#[inline(..)]`. |
| generic method | Called with its type and const parameters spelled out when no argument fixes them. |
| `self: Box<Self>` | The enum is moved out of its box and a `deref` variant's own `Box<Payload>` is handed over; a plain variant is an error, since boxing it again would allocate. |
| `self: Pin<&Self>`, `self: Pin<&mut Self>` | The enum is unpinned, matched, and the payload pinned again, which needs every payload to be `Unpin`. |
| default body | Ignored: forwarded like any other method. |
| `async fn`, `unsafe fn` | Arms are awaited; calls are wrapped in `unsafe { }`. |
| `unsafe trait` | An `unsafe impl`. |
| `-> impl Future<Output = T>` | The `match` is written in one `async move` block and every arm is awaited, as for `async fn`; `Send` and any other bound are checked against that block. On a struct the one arm returns the field's future as it is, unless its output is rewrapped or converted. |
| other `-> impl Trait` | An error on an enum, whose arms would each return a different opaque type; forwarded as it is on a struct, whose one arm returns one. |
| argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`; different variants give the `mismatch` expression. |
| return `Self` | Wrapped back into the same variant; `Option<Self>` and `Result<Self, E>` are mapped, under any path, `io::Result<Self>` included. `Self` anywhere else in a return type is an error. |
| method without a receiver | Left to the trait's default body, or written in a `#[dispatch] impl Trait for Enum` block; an error when it has neither. |
| `#[cfg]` on a method, an associated item or a variant | Carried onto the generated item or arm, and evaluated in the enum's crate. An associated type or const comes from the first variant without a `#[cfg]`. |
| `#[allow(..)]` on a method or an associated item | Carried onto the generated item, which repeats the signature the lint was allowed on. `warn`, `deny` and `expect` stay the trait crate's, and so does an `#[allow]` another macro added. |

## Limits

- Every variant a trait is dispatched to holds exactly one unnamed field,
  `Name(Payload)`, and every payload implements the trait. A variant of
  another shape is reported under its own `#[cfg]`, so a variant compiled
  out, such as a test-only mock, is no error; an enum that only forwards
  inherent methods may hold such variants, which take the method's body.
- An enum cannot dispatch a return-position `impl Trait`, at any depth of
  the return type, except `impl Future<Output = T>`, which is awaited like
  an `async fn`; a struct forwards any.
- A `deref` variant cannot take part in a method returning `Self`: there is
  no way to put the result back behind the pointer.
- A generic associated type is declared by the enum, in a partial impl
  block; it cannot come from the first variant.
- A projection through a trait parameter with several bounds, `F::Value`
  with `F: Format + Clone`, cannot be qualified at the enum; the trait
  writes `<F as Format>::Value`.
- An associated type or const a partial impl block declares under `#[cfg]`
  is an error: the forwarded methods convert into the type in every build.
  Put the whole block under the `cfg`, with a second block for the other
  builds.
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
- The types in the trait's signatures are written into the enum's module
  and must be in scope there, as for a hand-written impl; `crate::` paths
  are rewritten to the trait's crate, `super::` and `self::` paths are not.
  They must also be nameable from there: a private alias or type in a
  signature limits the trait to its own crate.
- An associated const of a type other than an integer, `bool`, `char` or
  `&str`, or on a generic enum, is taken from the first variant without a
  check.
- One `macro_rules!` may emit a `#[dispatchable]` trait in several modules
  when at least one token of the trait, its name or a method's, comes from
  the invocation. A trait written out in full in the macro's body gets the
  same carrier macro name at every invocation, and rustc reports it as
  defined multiple times, unless each invocation passes its own
  `#[dispatchable(name = ..)]`.

Every unsupported shape is reported as one sentence on the offending trait
item or variant; see `tests/ui` for all of them. A rejected enum is still
emitted, so the derives, uses and impl blocks around it compile and that
one error stands alone.

"cannot find macro `Shape` in this scope" at the enum means the trait is not
marked `#[dispatchable]`, or the trait, which carries the macro, is not
imported where the enum is. A `std::`, `core::` or `alloc::` path in
`#[dispatch]` is reported directly, with the `whichever::foreign`
declaration or the `remote` restatement to use instead. "could not find
`Trait` in `other`" on `#[dispatch(other::Trait)]` means the same for a
trait of another crate: restate it with `remote`. The same words on a
partial block over another crate's enum mean the enum has no
`#[whichever::dispatch]`, which only its own crate can add.

One `macro_rules!` that emits the same `pub` trait, written out in full,
at two places gets the same carrier macro name twice; `#[dispatchable(name =
tokio)]` names the macro explicitly and tells the two apart.

## Compatibility

`no_std`, with no dependencies beyond the proc-macro crate. The minimum
supported Rust version is 1.88. Both the crate that owns the trait and the
crate that owns the enum depend on `whichever`; renaming the dependency is
fine.

`#[dispatchable]` on a `pub` trait adds one `#[doc(hidden)]` exported macro,
the carrier macro, to the crate root, named after the trait and the position
of its tokens,
which is how another crate's enum reaches the trait's signatures. A trait
of any other visibility keeps its macro in its own module and adds nothing
to the crate root.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
