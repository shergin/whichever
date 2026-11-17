# Options

Options on the enum go inside `#[dispatch(..)]`, after the trait path,
as in `#[whichever::dispatch(Storage, Error = StorageError)]`. A few
options go on a variant, on a trait method or on a marked field instead.

An override, a `mismatch` and a `via(..)` route belong to the trait
they follow. Write them after the trait path. If you write one on an
attribute that lists no trait, you get an error on the option, because no
impl would read it there. For a trait implemented by a
[partial impl block](partial-impl-blocks.md), these options go on the
block's attribute instead.

When the enum lists exactly one trait on its own attributes, the option
is still read beside that trait, so the impl is the one you meant and the
error stands alone. With two or more traits, or with none (a partial impl
block), it is not read anywhere. A name close to one of the enum's own
options, such as `form = false`, gets "did you mean `from = ..`?" instead,
and is not read either.

| Option | Where you write it | What it does |
| --- | --- | --- |
| [`Name = Type`](#name--type-override-an-associated-type) | on the enum, after the trait path | Fixes an associated type; results are converted into it. |
| [`Name = value`](#name--value-override-an-associated-const) | on the enum, after the trait path | Fixes an associated const. |
| [`mismatch(method = expr, ..)`](#mismatch-what-to-return-when-two-variants-differ) | on the enum, after the trait path | What `method` returns when `self` and a `Self` argument hold different variants. One entry per method. |
| [`mismatch = expr`](#mismatch-what-to-return-when-two-variants-differ) | on a trait method | The same, decided once by the trait's author. |
| [`from = false`](#from--false-no-from-impls) | on the enum, after the trait path | No `From<Payload>` impls. |
| [`try_from = true`, `ref`, `ref_mut`](#try_from-get-the-payload-back-out) | on the enum, after the trait path | Adds `TryFrom<Enum> for Payload`, and the by-reference forms. |
| [`into = Target`](#into--target-convert-the-enum-into-another-type) | on the enum, after the trait path | Adds `From<Enum> for Target`. |
| [`via(Variant = route, ..)`](#viavariant--route-a-route-for-one-trait) | on the enum, after the trait path | A route through `Variant` for this trait only. One entry per variant. |
| [`from = false`](#options-on-a-variant) | on a variant | Leaves the variant out of `From` and `TryFrom`. |
| [`try_from = true`](#options-on-a-variant) | on a variant | Adds `TryFrom` for this variant's payload alone. |
| [`deref`](#options-on-a-variant) | on a variant, or on the marked field | Calls the trait on what the payload dereferences to. |
| [`via = route`](#options-on-a-variant) | on a variant, or on the marked field | Calls the trait on a field of the payload, or on what a method returns. |
| [`#[dispatch]`](#the-marked-field) | on one field of a variant | Picks the field to forward to, in a variant with several fields or a named one. |
| [`remote = path`](restatements.md#enums-from-other-crates) | on the enum or the struct | Restates an enum or a struct another crate defines, so you can dispatch traits on it. |
| [`#[dispatch(Trait)]`](structs.md#several-traits-several-fields) | on a field of a struct | Makes the field the target of `Trait`. |

## `Name = Type`: override an associated type

Without this option, the enum takes each associated type from its first
variant. `Name = Type` sets it on the enum instead. Each variant's own
type then only has to convert into it, so variants with different error
types can share one enum:

```rust
#[whichever::dispatchable]
pub trait Storage {
    type Error;
    fn load(&self, key: &str) -> Result<u32, Self::Error>;
}

impl Storage for Memory {
    type Error = StorageError;
    // ..
}

impl Storage for Disk {
    type Error = io::Error; // and `io::Error: Into<StorageError>`
    // ..
}

#[whichever::dispatch(Storage, Error = StorageError)]
pub enum AnyStorage {
    Memory(Memory),
    Disk(Disk),
}
```

`AnyStorage::load` returns `Result<u32, StorageError>`. The `Disk` arm
converts its `io::Error` on the way out.

What a method returns is converted:

- `Self::Name` is converted with `Into`.
- `Result<_, Self::Name>` is converted with `map_err(Into::into)`.
- `Option<Self::Name>`, `Result<Self::Name, E>` and
  `Result<Self, Self::Name>` are mapped the same way.
- A tuple holding `Self::Name` is taken apart and each element converted.

### Arguments are not converted

An argument that names the type is handed to the payload as it is. That
includes a closure over the type. So a method that takes `Self::Name`
compiles only where every payload's own `Name` already is the override's
type.

For example, `Item = u8` over payloads whose `Item` is `u8` forwards
every method. But `Item = Wide` over a `Range<u32>` and a `Range<u64>`
stops at the methods of `foreign::Iterator` that take a closure over
`Self::Item`: `fold`, `any`, `max_by` and the rest. There you get a type
error from rustc in that arm. A hand-written arm would need a closure that converts each item,
and the attribute does not write closures.

You have two ways out:

- [Restate the trait](restatements.md#payloads-with-different-items)
  with only the methods you want to forward, as that page shows for
  `Iterator`. The other methods then use
  the trait's default.
- For a method or two, leave each one to the trait's default with
  `#[dispatch(default)]` in a
  [partial impl block](partial-impl-blocks.md).

## `Name = value`: override an associated const

`Name = value` fixes the associated const `Name`, instead of taking it
from the first variant. Use it for a const that differs per payload by
design, such as a kind discriminator:

```rust
#[whichever::dispatch(Proof, KIND = "any")]
pub enum AnyProof {
    Encoding(EncodingProof), // KIND = "encoding"
    Ordering(OrderingProof), // KIND = "ordering"
}
```

Without the option, the enum would take `"encoding"` from the first
variant, and the
[agreement check](what-is-generated.md#associated-consts-and-the-agreement-check)
would give you an error on `Ordering`.

## `mismatch`: what to return when two variants differ

A method with a `Self` argument, such as `fn congruent(&self, other:
&Self) -> bool`, matches `self` and `other` together. When they hold
different variants, there is no payload method to call, and the method
returns the mismatch expression instead.

The trait's author can decide it once, on the trait method:

```rust
#[whichever::dispatchable]
pub trait Shape {
    /// Two different kinds of shape are never congruent.
    #[dispatch(mismatch = false)]
    fn congruent(&self, other: &Self) -> bool;
}
```

Or you decide it on the enum, for each method by name:

```rust
#[whichever::dispatch(Shape, mismatch(congruent = false))]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

Without either, you get an error naming the method. An enum with one
variant is the exception: it has no second variant to meet, and gets no
mismatch arm.

### What the expression can use

- The method's parameters, a `Self` argument included.
- `self`, when the method takes `&self`. Under `&mut self` and `self`,
  the receiver is moved into the `match`, so the expression cannot use it.
- On the trait method only: the method's own generic parameters, as in
  `#[dispatch(mismatch = T::default())]`.

### When a generic parameter is renamed

When the impl renames a method's generic parameter (see
[What is generated](what-is-generated.md#renamed-generic-parameters)),
the trait method's expression follows the new name: `T::default()`
becomes `T2::default()`.

- The rename reaches inside a macro's arguments, and into the
  placeholders of a format macro's format string: `format!("{N}")`.
- Any other string is text and stays as you wrote it. So does the input
  of `stringify!` and `cfg!`.
- A macro of your own that quotes its input as text cannot be told apart
  from one that runs it, so its input is renamed like code.

### Where paths resolve

A path in the expression on the trait method resolves where the enum is,
as the types in the signatures do. So a helper next to the trait is
spelled from `crate::`, as in `crate::shapes::never_congruent()`.

### Methods under `#[cfg]`

A method the trait declares under a `#[cfg]` takes its mismatch on the
trait method, which travels with the method under its `cfg`. On the enum,
`mismatch(method = ..)` would refer to nothing in a build where the `cfg`
is off, and you get an error.

### Methods that return a future

- Where the arms are awaited, the expression is the future's output. This
  is the case for an `async fn`, an `impl Future`, and a boxed future
  whose output is converted.
- Where a boxed future is forwarded as it is, the expression is a value of
  the declared future type.

## `from = false`: no `From` impls

By default the enum gets `From<Payload>` for each variant.
`from = false` turns them all off:

```rust
#[whichever::dispatch(Shape, from = false)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
```

- A payload type that two variants hold gets no `From` either way: there
  would be two impls of the same `From`, one per variant.
- To leave out one payload, put `#[dispatch(from = false)]` on its
  variant instead (see [Options on a variant](#options-on-a-variant)). Do
  this when you write that `From` by hand or a derive supplies it, because
  a second impl of the same `From` is a conflict.
- Some payloads cannot have a `From` at all, and you get an error that
  asks for `from = false`. A payload that is a type parameter of the
  enum, `Iter(I)`, would overlap the other variants' `From` impls. A
  payload that is an associated type of a type parameter, `Out(S::Out)`
  or `Out(<S as Source>::Out)`, may be any type, the enum included, so its
  `From` would overlap `From<T> for T`, even with one variant. A
  projection from a concrete type, `<Fixed as Source>::Out`, is fine: it
  gets its `From`.

## `try_from`: get the payload back out

`try_from = true` adds `TryFrom<Enum> for Payload` for every payload type:

```rust
#[derive(Debug)]
#[whichever::dispatch(Shape, try_from = true)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}

let circle = Circle::try_from(shape).unwrap();
```

The error type is the enum itself, so when the value holds another
variant, you get it back. That is why `.unwrap()` on the result needs the
enum to be `Debug`.

Two wider forms add the by-reference impls. The words are the ones
`derive_more` uses.

| Option | Adds |
| --- | --- |
| `try_from = true` | `TryFrom<Enum> for Payload` |
| `try_from = ref` | the above, and `TryFrom<&Enum> for &Payload` |
| `try_from = ref_mut` | both of the above, and `TryFrom<&mut Enum> for &mut Payload` |

Each by-reference impl hands the reference back as the error.

## `into = Target`: convert the enum into another type

`into = Target` adds `From<Enum> for Target`. Each arm converts its
payload through the payload's own `Into<Target>`. Say every message type
converts into one wire type; then the enum converts in one line too:

```rust
#[whichever::dispatch(Message, into = Wire)]
pub enum AnyMessage {
    Ping(Ping), // impl From<Ping> for Wire
    Data(Data), // impl From<Data> for Wire
}

let wire: Wire = message.into();
```

- You can repeat the option for several targets.
- Every variant has to hold a payload and take part in the conversions.

## Options on a variant

Four options go on a variant, in `#[dispatch(..)]` above it:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape<'a> {
    #[dispatch(deref)]
    Boxed(Box<Circle>),
    #[dispatch(deref)]
    Borrowed(&'a Square),
    #[dispatch(via = .shape)]
    Labelled(Labelled),       // struct Labelled { shape: Square, label: String }
    #[dispatch(from = false)]
    Name(String),             // String implements `Shape` here
    Plain(Square),
}
```

- **`from = false`** leaves the variant out of the `From` and `TryFrom`
  impls. Use it for a payload such as `String` that should not convert
  into the enum.
- **`try_from = true`** adds the `TryFrom` impl for this variant's payload
  alone.
- **`deref`** calls the trait on what the payload dereferences to. So a
  `Box<T>`, `Rc<T>`, `Arc<T>`, `&T` or `&mut T` payload is forwarded like
  a plain `T`.
- **`via = route`** calls the trait on a field of the payload, or on what
  a method of it returns.

`deref` and `via` are routes. [Reaching the target](routes.md) has
everything about them.

### The marked field

A variant with several fields, or with one named field, marks the field
to forward to with `#[dispatch]`:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Tagged(#[dispatch] Square, u32),
    Row { id: u32, #[dispatch] cell: Square },
}
```

Without the mark, you get an error on the variant.

The marked field takes `deref` and `via = route`, and nothing else:

- The conversion options (`from`, `try_from`) go on the variant, because
  the conversions belong to the variant.
- The field cannot list a trait, because the variant is the target of
  every trait its enum dispatches.

## `via(Variant = route)`: a route for one trait

A route on a variant applies to every trait. When each trait needs a
different route through the same variant, write the route on the enum's
attribute for that trait:

```rust
#[whichever::dispatch(Area, via(Pair = .1))]
#[whichever::dispatch(Named, via(Pair = .0))]
pub enum Any {
    Pair((Label, Square)),
    Plain(Square),
}
```

For that trait, it wins over the variant's own route.

## Dispatching several traits

Repeat the attribute, one trait per attribute. A supertrait is one more
attribute:

```rust
#[whichever::dispatch(Shape)]
#[whichever::dispatch(Drawable)]
pub enum AnyShape { /* .. */ }
```

The conversions belong to the enum, not to a trait. They are written
once, however many attributes the enum has. A conversion option on any
one attribute applies to all of them, so together they say one thing:

- **`from`**: `from = false` on any attribute leaves the `From` impls
  out. Two attributes may give `from` the same value. An attribute that
  gives it the other value contradicts the first, and you get an error.
- **`try_from`**: the forms add up, and the widest one on any attribute
  applies. So `try_from = false` next to another form cancels nothing,
  and is not an error.
- **`into`**: the targets of all the attributes are collected. A target
  listed twice, on two attributes or on one, would be the same `From`
  written twice, and you get an error.

## Generic traits

A generic trait is forwarded at one instantiation, which you write on the
enum:

```rust
#[whichever::dispatch(Encoder<Json>)]
pub enum AnyEncoder {
    Compact(Compact),
    Pretty(Pretty),
}
```

- **The enum's own parameters** work too:
  `#[whichever::dispatch(GridTrait<T>)]` on `enum Grid<T>`. The enum's
  parameters, bounds and `where` clause carry into the impl.
- **A parameter with a default** may be left out:
  `#[whichever::dispatch(Enc)]` for `trait Enc<F = Json>`, as
  `impl Enc for A` leaves it out.
- **Substitution**: type, lifetime and const parameters are replaced by
  their arguments in every signature.

### Projections

A projection such as `F::Value` becomes `<Json as Format>::Value`, when
`F` has exactly one bound. It is written with the bound's generic
arguments, and without any constraint on an associated type. So
`F: Format<Text, Value = u64>` gives `<Json as Format<Text>>::Value`.

With several bounds, the attribute cannot tell which trait `Value` comes
from. Then the trait has to spell it out: `<F as Format>::Value`.

### Renamed generic parameters

A method's own generic parameter may have the same name as something the
impl gives a meaning to. Then it is renamed first, so the two stay apart.
That happens when it has the name of:

- one of the enum's parameters: `fn encode<T>` in `Encoder<T>` on
  `enum E<T>` becomes `encode<T2>`;
- an argument given to the trait: `fn encode<Json>` in `Encoder<Json>`
  becomes `encode<Json2>`.

The new name adds the first number that neither the impl nor the trait
uses. [What is generated](what-is-generated.md#renamed-generic-parameters)
has the full rule.

### Payloads that are type parameters

A payload that is a type parameter of the enum gives the impl a bound on
that parameter, which is the bound you would write by hand:

- `Raw(Raw)` on `enum Lazy<Raw, Parsed>` gives the bound `Raw: Trait`.
- So does `Box<Raw>` under `deref`.
- A struct's field works the same way: `struct Wrapper<T>(T)` gets
  `impl<T: Shape> Shape for Wrapper<T>`.
