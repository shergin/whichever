# Forwarding from a struct field

A [dispatch enum](glossary.md#dispatch-enum) (an enum with `#[dispatch]`
on it) is often private. It sits behind a public struct, and the struct
implements the same trait by hand, passing each call to the enum.

That hand-written half usually forwards only the required methods. So if
a payload overrides a provided method (a method with a default body), the
override never reaches the public type: the struct runs the default.

Put `#[dispatch(Trait)]` on the struct instead. It forwards to the
struct's one field, or to the field marked `#[dispatch]`, and it forwards
every method, provided ones included:

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

`StreamFlowCtrl` has two fields, so `#[dispatch]` marks the one that every
call goes to. The `hooks` field is left alone.

## Any implementor will do

The field does not have to be a dispatch enum. Any type that implements
the trait works, so the same attribute also replaces a field-delegation
derive:

```rust
#[whichever::dispatch(Addresses)]
pub struct Beacon {
    #[dispatch]
    header: ManagementHeader,
    body: Vec<u8>,
}
```

## Several traits, several fields

A field may list the traits it is the [target](glossary.md#target) of.
The field marked with a bare `#[dispatch]` takes every trait that no
other field lists:

```rust
#[whichever::dispatch(Rom)]
#[whichever::dispatch(Interrupt)]
#[whichever::dispatch(Named)]
pub struct Mapper {
    #[dispatch(Rom)]
    rom: Cartridge,
    #[dispatch(Interrupt)]
    signals: Signals,
    #[dispatch]
    label: Label,       // `Named`, and any other trait
}
```

### Where a trait's impl lives

A trait's impl has one home. It is either the struct's attribute, which
lists every trait whose impl it writes, or a
[partial impl block](partial-impl-blocks.md). Whichever home writes the
impl, the trait goes to the field that lists it.

### How a field lists a trait

A field lists a trait exactly as the trait's home spells it: as the
struct's attribute spells it, or as the block's header spells it.

- **Generic arguments are part of the name.** So two instantiations of one
  generic trait, `Encoder<Json>` and `Encoder<Xml>`, can go to two fields.
- **The module path is part of the name.** So two traits of one name from
  two modules can go to two fields.
- **Start from `crate`, `self`, `super` or the root** where the home's
  path starts there, as in `#[dispatch(crate::traits::Catalog)]`.

## When the impl needs its own header

Sometimes the impl needs a header the attribute cannot write, such as a
bound that ties one field's type to another's. Then the trait's home is a
partial impl block, and the field still lists the trait:

```rust
#[whichever::dispatch(Named)]
pub struct Env<S, C, N> {
    pub syntax: S,
    #[dispatch(Catalog)]
    pub catalog: C,
    #[dispatch]
    pub named: N,
}

impl<S: Sorts, C, N> Sorts for Env<S, C, N> {
    type Extractor = S::Extractor;
}

#[whichever::dispatch]
impl<S: Sorts, C: Catalog<Extractor = S::Extractor>, N> Catalog for Env<S, C, N> {}
```

Notice that the struct's attribute lists only `Named`, and leaves
`Catalog` out: the block is the impl of `Catalog`. The `Sorts` impl is
written by hand, because `Catalog` needs it as a supertrait.

- **A generic trait.** The block writes the trait's parameter, as in
  `impl<F, E: Encoder<F>, N> Encoder<F> for Env<E, N> {}`, and the field
  lists the trait the way the header spells it: `#[dispatch(Encoder<F>)]`.
- **Methods of the block.** A method marked `#[dispatch(*)]` goes to the
  field that lists the trait. So does a method that names that field, as
  in `#[dispatch(catalog)]`.
- **Naming another field is an error.** A trait goes to one field. To
  send it to another field, list the trait on that field instead. The
  error says:

  ```text
  error: `named` is a field of `Env`, but this trait is forwarded to `catalog`, and a trait is forwarded to one field; name the trait on `named`, `#[dispatch(Trait)]`, to forward it there
  ```

## Spelling: one name, two ways

Here the struct's attribute and the field spell one trait two ways:

```rust
#[whichever::dispatch(traits::Catalog)]
struct Env {
    #[dispatch(Catalog)]
    catalog: Books,
    #[dispatch]
    named: Label,
}
```

You get an error:

```text
error: `Catalog` on `catalog` and `traits::Catalog` on the struct spell one trait's name two ways, so `catalog` is not the field `traits::Catalog` is forwarded to; a field names a trait as the struct's attribute spells it
```

The rule: a field lists a trait as the trait's home spells it. When no
field lists the home's trait in that spelling, but a field lists it in
another, you get this error. Another spelling is:

- more or fewer path segments, as in `Catalog` and `traits::Catalog`, or
- with or without its generic arguments, as in `Encoder` and `Encoder<F>`.

Without the error, the home's trait would quietly go to the bare field,
`named`, instead. You get it at either home, the struct's attribute or a
block.

The impl is still written for the field that was meant. So this error is
the only one you see, and the calls of the trait's methods compile.

A field's mark that no home claims is not an error. It forwards nothing,
and nothing says so, because the struct does not see its blocks. The
first use of the trait then tells you that the impl is missing.

## Functions and generic associated types

A struct's target has a type the struct names: its one field, or the
marked one, reached directly or through `deref`. So what needs no value
comes from that type, as you would write it by hand:

- **A trait method without a receiver** is the field type's,
  `<Square as Shape>::kind()`. That includes provided methods, so the
  field's override of one is what runs.
- **`-> Self` and `Option<Self>`** are built around the result, where
  the struct has no other field: `Wrapper(<Square as Shape>::unit())`.
- **A generic associated type** is the field type's too:
  `type Side<'a> = <Square as Shape>::Side<'a> where Self: 'a;`. A
  parameter named like one of the struct's is renamed, so on
  `struct Generic<T>(T)` you get
  `type Paired<T2> = <T as Shape>::Paired<T2>;`.
- **An inherent method without a receiver**, marked in an inherent
  block, is a path call on the field's type. It finds an inherent
  function before a trait's, so a constructor is one line:

```rust
#[whichever::dispatch]
pub struct Arith256(U256);

#[whichever::dispatch]
impl Arith256 {
    #[dispatch(*, call = from)]
    pub fn from_u64(v: u64) -> Self; // `Self(<U256>::from(v))`
}
```

`call` and `with` work as on any inherent method. `async fn`,
`unsafe fn` and generic functions are forwarded too.

### What the field's type cannot be asked

Some functions still cannot come from the field's type:

- one that takes a `Self` argument, because a `Self` argument is
  unwrapped beside a receiver only;
- one over a field that holds a trait object, which has no function to
  call without a value;
- one over a field reached through a field or a method, whose type the
  struct does not say.

There, a provided method is left to the trait's default, and a required
one gets an error: give it a default body, or write it in a
[partial impl block](partial-impl-blocks.md). An inherent one gets an
error that asks for a body. A generic associated type over such a field
is declared in a partial impl block. `field` and `value` on an inherent
method without a receiver are an error: there is no value to read.

## A trait in some builds only

A wrapper may implement a trait on some targets only, or with some
features only. Put the trait's attribute under `cfg_attr`, mark the field
in every build, and add a bare `#[whichever::dispatch]` beside it:

```rust
#[whichever::dispatch]
#[cfg_attr(feature = "metrics", whichever::dispatch(Shape))]
pub struct Labelled {
    #[dispatch]
    shape: Square,
    pub label: String,
}
```

With the feature on, the struct forwards `Shape` to `shape`. With it off,
there is no impl, and the bare attribute still takes the mark off the
field, so the `#[dispatch]` there is no unknown attribute.

Why not put the mark under `cfg_attr` too? An attribute macro is handed
its item before any `cfg_attr` inside the item is decided, so a mark
under `cfg_attr` would be read in no build. You get an error on such a
mark, saying what to write instead.

The other way is a [partial impl block](partial-impl-blocks.md) under
`#[cfg(..)]`, with the struct registered by a bare attribute:
`#[cfg(feature = "metrics")] #[whichever::dispatch] impl Shape for
Labelled {}`.

## What is different from an enum

- **`#[dispatch(deref)]` on the field** reaches through a `Box` or another
  pointer, as it does on a variant. `#[dispatch(via = .source(), via_mut =
  .source_mut())]` reaches through a getter pair of the field's type, the
  mutable getter for the methods that borrow mutably; see
  [A route for a mutable borrow](routes.md#a-route-for-a-mutable-borrow).
  A trait's own routes through the field go after the trait,
  `via(field = route)` and `via_mut(field = route)`, on the struct's
  attribute or on a partial impl block's.
- **A method returning `Self`** is rebuilt only around a struct with no
  other field, because the macro has no values for the others. On a struct
  with other fields it is an error, and the method goes in a
  [partial impl block](partial-impl-blocks.md). A partial impl block works
  on a struct as it does on an enum.
- **`Option<Self>` and `Result<Self, E>`** are mapped with a path, as in
  `.map(Wrapper)`. A tuple struct has such a path; a struct with a named
  field does not. So on `struct Named { shape: Square }` they are an error
  too, and the tuple form `struct Named(Square);` avoids it. A plain
  `-> Self` on the named form is fine: it is rebuilt with braces, as it is
  for a variant whose one field is named and marked.
- **A return-position `impl Trait` is forwarded as it is.** A struct's
  `match` has one arm, so there is only one hidden type. An enum cannot do
  this.
- **A struct gets no `From` impls.**
- **`Self` arguments need no `mismatch`.** Two values of the struct always
  hold the same field, and the argument is unwrapped to it.
- **Another crate's struct is restated with `remote = path`**, as another
  crate's enum is, and the restatement lists only the fields its impls
  forward to. See
  [Structs from other crates](restatements.md#structs-from-other-crates).
