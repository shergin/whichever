# Partial impl blocks: write some methods yourself

Forwarding does not fit every trait method. On an enum, a method without
a receiver, such as a constructor, has no variant to forward to. And
sometimes the enum should return something different from every payload.
(A struct forwards a method without a receiver to its field's type; see
[Forwarding from a struct field](structs.md#functions-and-generic-associated-types).)

For these, write a
[partial impl block](glossary.md#partial-impl-block): a trait impl block
with `#[dispatch]` on it. You write the items the enum handles itself,
and the rest of the trait is [forwarded](glossary.md#forwarding) around
them:

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

`from_handler` is yours. Every other method of `HandlerTrait` is
forwarded to `TextHandler` or `MapHandler`, as if the enum had
`#[dispatch(HandlerTrait)]`.

On this page:

- [The basics](#the-basics)
- [Write a method in full](#write-a-method-in-full),
  or [mark it instead](#mark-a-method-instead-of-writing-it)
- [Forward to every variant but a few](#forward-to-every-variant-but-a-few)
- [Reach unit variants and variants without an impl](#reach-unit-variants-and-variants-without-an-impl)
- [Marks under `#[cfg]`](#marks-under-cfg)
- [Attributes on the methods](#attributes-on-the-methods)
- [The impl header](#the-impl-header)
- [Which names must be in scope](#which-names-must-be-in-scope),
  [attributes on the block](#attributes-on-the-block),
  [what the block cannot hold](#what-the-block-cannot-hold)
- [Call the payloads from a method you wrote](#call-the-payloads-from-a-method-you-wrote)
- [On a struct](#on-a-struct)

## The basics

- **The enum does not list the trait in its own attribute.** The block
  is the impl. If you list the trait in both places, you get an error on
  the block.
- **The trait's options go on the block's attribute**:
  [overrides](glossary.md#override) such as `Name = Type`, and
  [`mismatch`](options.md), as in
  `#[whichever::dispatch(mismatch(same = false))] impl Same for Any {}`.
  On the enum's own attribute, which lists no trait, they are an error:
  no impl would read them there.
- **An associated type written in the block is an override too**, as if
  you had written `Name = Type` on the attribute.
- **An enum's generic associated type is declared here**, and so is a
  struct's whose field is reached through a route. No first variant can
  supply one, so you declare it in the block, and each payload's own type
  converts into it through `Into`, as for any override:

  ```rust
  type Item<'a> = AnyItem<'a> where Self: 'a;
  ```
- **The block has to be in the enum's crate**, as an
  [inherent block](inherent-methods.md) does.

## Write a method in full

A method you write in the block replaces the forwarding for every
variant. This includes provided methods, so it is also the way to leave
out a payload's version of a method on purpose.

A method written under `#[cfg]` replaces the forwarding where the `cfg`
is on, and is forwarded where it is off. That is what you would write by
hand: two versions of the method under opposite `cfg`s.

## Mark a method instead of writing it

A method in the block may list the variants it is forwarded to, as an
[inherent method](inherent-methods.md) does. A [mark](glossary.md#mark)
is `#[dispatch(..)]` on a method of the block, and a method with one is
marked. Its body is then the arm for every other variant:

```rust
#[whichever::dispatch]
impl whichever::foreign::Display for Category {
    /// Every other kind of category prints its own name.
    #[dispatch(Wep, Wpa, Wpa2)]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Others")
    }
}
```

`Wep`, `Wpa` and `Wpa2` are forwarded; every other variant prints
`Others`.

The body also runs when `self` and a `Self` argument hold two different
variants.

There are three other marks:

- **`#[dispatch(*)]` on a method without a body** is the plain
  forwarding's arms, written out.
- **`#[dispatch(*)]` on a method with a body** holds the arms the enum
  handles itself. See the next section.
- **`#[dispatch(default)]` on a method without a body** leaves the method
  to the trait's default. It is neither forwarded nor written. Over a
  [restatement](restatements.md), that is the real trait's default,
  which the restatement cannot see.

## Forward to every variant but a few

When the enum handles only a few variants itself, `#[dispatch(*)]` with a
body writes the list the other way round. The body is one `match self`
that holds the arms the enum handles itself, and every variant those
arms do not name is forwarded:

```rust
#[whichever::dispatch]
impl Encode for Codec {
    /// An unknown codec is written back as it was read.
    #[dispatch(*)]
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), Error> {
        match self {
            Self::Unknown(kind, data) => {
                buf.extend_from_slice(kind);
                buf.extend_from_slice(data);
                Ok(())
            }
        }
    }
}
```

The method is the `match` you would write by hand, minus the arms that
only forward.

### The generated `match`

The generated `match` holds a forwarding arm for each other variant, in
the enum's order, then your arms as you wrote them. It has no wildcard
arm:

- A variant you add to the enum later is forwarded, with no second edit.
- A variant that holds no payload, and has no arm of yours, gives an
  error on the method. The error names the arm to write.

### Rules for your arms

- **An arm names its variants by path**, as `Self::Unknown(..)`. It
  may do so through a guard, an or-pattern, a reference or a binding
  `name @ ..`.
- **A variant an arm names is not forwarded**, whether the arm has a
  guard or not.
- **An arm that names no variant gives an error**, such as `_` or a
  plain binding. A wildcard would mean "every other variant", and that
  is what a method with a list of variants and a body already says.
- **The arms are the method's own.** They are written as they stand, in
  the declared return type, and read the fields their pattern binds.
- **In an `async fn`**, your arms sit where the forwarding arms sit:
  inside the future.
- **An arm under `#[cfg]`** handles its variant where the `cfg` is on.
  Where it is off, the variant is forwarded.

### When you cannot write your own arms

Two kinds of method cannot take arms of the enum's own under `*`:

- A trait method with a `Self` argument, because the argument is matched
  beside `self`, and your arm is written for `self` alone.
- A method over an enum whose every variant is under `#[cfg]`, because
  it is matched as `match *self`.

For both, list the forwarded variants in `#[dispatch(..)]` and give the
method a body for the rest.

## Reach unit variants and variants without an impl

This is how a trait reaches:

- a unit variant,
- a variant whose payload has no impl of the trait,
- a variant the enum handles itself, whether it holds a payload or not.

The method lists the other variants, or, under `*`, its body holds the
arms for these ones.

## Marks under `#[cfg]`

A mark can sit under a `#[cfg]`:

```rust
#[whichever::dispatch]
impl Shape for AnyShape {
    #[cfg(feature = "fancy")]
    #[dispatch(Circle)]
    fn name(&self) -> &'static str {
        "a shape"
    }
}
```

With the feature on, `name` is forwarded to `Circle`, and every other
variant returns `"a shape"`. With it off, `name` is forwarded to every
variant, as if the block did not mention it.

This is the rule of a
[method written under `#[cfg]`](#write-a-method-in-full), and
`#[dispatch(default)]` follows it too:

- Where the `cfg` is on, the mark applies: the method is forwarded as
  marked, or, under `#[dispatch(default)]`, left to the trait's default.
- Where it is off, a mark of the same name without `#[cfg]` applies, if
  there is one. If there is none, the method is forwarded to every
  variant.

A variant that no method can be forwarded to, such as a unit variant,
gives an error in the builds in which some method is forwarded to every
variant. In other builds, it gives none. So a mark under the trait
method's own `#[cfg]` gives no error: in every build that has the
method, the mark is on, and the method is never forwarded to every
variant.

A mark under `cfg_attr`, `#[cfg_attr(feature = "fancy", dispatch(Circle))]`,
is an error. The attribute is handed the block before rustc decides a
`cfg_attr` inside it, so the mark would be read in no build. Write the
method twice instead, under `#[cfg(..)]` with the mark and under
`#[cfg(not(..))]` without it.

## Attributes on the methods

A method you write or mark in the block is yours, and so are its
attributes:

- It carries what you write on it.
- It carries nothing of the trait method's own attributes, just as a
  hand-written impl inherits nothing from the trait's declaration.

Which of them gets `#[inline]` depends on its kind:

- A method you mark to be forwarded gets `#[inline]` unless you wrote an
  `#[inline(..)]` on it yourself. The attribute writes its `match`, so it
  adds what it adds to every method it writes.
- A method you write in full is emitted as you wrote it, with your
  attributes and nothing added. If you want it `#[inline]`, write that on
  it, as you would in a hand-written impl.
- A method marked `#[dispatch(default)]` is not in the impl at all: the
  trait's default runs.

Only the methods the block leaves to plain forwarding, whose signatures
the attribute writes, carry the trait method's `#[inline(..)]` and
`#[allow]`. That includes an `#[inline(..)]` written on a required trait
method, which `#[dispatchable]` keeps for the forwarding methods and
takes off the trait's own method, since rustc rejects it there. One
written under `cfg_attr` is not carried.

## The impl header

The block's header, generics and bounds included, becomes the impl's
header. It may refer to the enum's parameters however it likes:
`impl<T, Ty> Get<T> for ImmOrNode<Ty>`.

So an empty block is how the impl gets a header that the enum's
attribute cannot write: generics the enum does not have, bounds, or one
instantiation. A trait generic over its own parameter is implemented for
every `F`:

```rust
#[whichever::dispatch]
impl<F: Format> Encoder<F> for AnyEncoder {}
```

The other cases:

- A trait with a lifetime the enum lacks:
  `impl<'r> Renderable<'r> for Node {}`.
- A trait implemented for one instantiation of a generic enum:
  `impl ColNames for Lazy<Raw<'static>, Parsed> {}`.
- A bound on a type parameter that the trait's own `Param: Trait` bound
  does not cover: `impl<T: Shape + Clone> Shape for Wrapper<T> {}`.
- A bound for a payload that only mentions a parameter, such as
  `Wrapped(Boxed<T>)`. The automatic bound is a bare parameter's alone:
  `Raw(T)` gets `T: Trait`, and `Boxed<T>` gets nothing, since what its
  own impl asks of `T` is not written in the enum. Write the header the
  way you would by hand: `impl<T: Shape> Shape for AnyShape<T> {}`, or
  `impl<T> Shape for Labelled<T> where Boxed<T>: Shape {}`. The enum then
  lists no trait in its own attribute.

## Which names must be in scope

The forwarding arms refer to the payload types as the enum spells them.
So a block in another module needs those names in scope there.

It also works the other way round. When the trait's signature types are
not in scope where the enum is, write an empty block in a module where
they are, such as one holding `use trait_crate::*;`. That is the place to
write the impl.

## Attributes on the block

Outer attributes on the block are carried onto the impl. So
`#[typetag::serde(name = "..")]` goes on the block. On the trait, put
`#[whichever::dispatchable]` above `#[typetag::serde]`. Then `typetag`'s
own methods belong to the block and are never forwarded.

## What the block cannot hold

These items give an error in the block:

- **A macro invocation.** The attribute cannot tell the items the macro
  would write from the ones to forward, so write those items out in the
  block instead.
- **A method without a body and without a mark.**
- **A mark that does not parse.**
- **A mark with an inherent method's option**, such as `call`.
- **A second mark on a method that already has one.**
- **`#[dispatch(default)]` on a method with a body.**

Under `#[cfg]`, each of these gives an error only where the `cfg` is on.
Where it is off, the item is not in the block, as it would not be in a
hand-written one. The block is read without it, so the method a macro
would have written, or the method the item names, is forwarded.

## Call the payloads from a method you wrote

A method you write in the block may still need the payloads' result.
Have it call an inherent `#[dispatch(*, call = name)]` method beside it:

```rust
#[whichever::dispatch]
impl AnyShape {
    #[dispatch(*, call = describe)]
    fn payload_describe(&self) -> String;
}

#[whichever::dispatch]
impl Describe for AnyShape {
    fn describe(&self) -> String {
        format!("a shape: {}", self.payload_describe())
    }
}
```

## On a struct

A partial impl block over a [dispatch struct](structs.md) goes to the
field that lists the trait as the block's header spells it:

- `#[dispatch(Catalog)]` on the field, for
  `impl<..> Catalog for Env<..> {}`.
- `#[dispatch(Encoder<F>)]` on the field, for
  `impl<F, ..> Encoder<F> for Env<..> {}`.

If no field lists the trait, the block goes to the field marked with a
bare `#[dispatch]`, or to the only field, as the struct's attribute
does. See [Forwarding from a struct field](structs.md).

### Wrap some methods, forward the rest

A struct often wraps a few of a trait's methods, with a retry or a check
before the call, and forwards the others to its field as they are. You
write that as a dispatch struct with a partial impl block that holds the
wrapped methods:

```rust
#[whichever::dispatch]
pub struct Retrying {
    #[dispatch(deref)]
    inner: Box<dyn Store>,
    attempts: u32,
}

#[whichever::dispatch]
impl Store for Retrying {
    fn get(&self, key: &str) -> Result<u32, String> {
        retry(self.attempts, || self.inner.get(key))
    }
}
```

Every method the block does not hold is forwarded as it is, without the
wrapping. That includes a method added to the trait later. Nothing
reports it: the struct still builds, and its file does not change, so
the new method is not in the diff either. The call reaches the field,
which is right, but it is not retried or checked.

`#[deny(clippy::missing_trait_methods)]` on the block does not catch
this. The lint does not look at an impl a macro writes, and the
generated impl holds every method of the trait anyway, so the lint
would have nothing to report.

So when every new method of the trait has to be looked at before the
struct passes it on, write the impl by hand under
`#[deny(clippy::missing_trait_methods)]`. Then a new provided method
stops `cargo clippy` until you write it into the impl, plain or wrapped,
and a reviewer sees which. (A new required method stops the build
either way.) When the struct should pass every new method on as it is,
the partial impl block is the shorter impl, and it cannot forget one.
