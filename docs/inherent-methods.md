# Inherent methods

An enum often needs methods of its own, outside any trait, that only some
variants can provide. Only a circle has a radius; a square does not. By
hand, each such method is a `match` that passes the call on to the
variants that have the method and returns something else for the rest.

Put `#[dispatch]` on the enum's inherent `impl` block, an
[inherent block](glossary.md#inherent-block), and mark the methods to
forward:

```rust
#[whichever::dispatch]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}

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

Here is what to notice:

- A marked method is [forwarded](glossary.md#forwarding) to the method of
  the same name on the listed variants' [payloads](glossary.md#payload).
  `radius` calls `Circle::radius`.
- The body is the [fallback](glossary.md#fallback): what every other
  variant returns. A `Square` returns `None`.
- `*` means every variant, so `describe` needs no body.
- Methods without `#[dispatch(..)]` are left alone.

`radius` becomes the `match` you would write by hand:

```rust
pub fn radius(&self) -> Option<f64> {
    match self {
        Self::Circle(inner) => Some(inner.radius()),
        _ => None,
    }
}
```

The options change what each forwarding arm becomes:

| Option | The arm becomes |
| --- | --- |
| (none) | `inner.method(args)` |
| `with = path` | `path(inner.method(args))` |
| `call = name` | `inner.name(args)` |
| `field = name` | the field `inner.name`, borrowed or read |
| `value` | the target itself, borrowed or read |
| `via = route` | the call or place, on the target the route reaches |

On this page:

- [Choose the variants](#choose-the-variants)
- [Forward to every variant but a few](#forward-to-every-variant-but-a-few)
- [How the call is written](#how-the-call-is-written)
- [Options](#options)
- [Methods that return `Self`](#methods-that-return-self),
  [`async` methods](#async-methods),
  [`Self` arguments](#self-arguments)
- [Methods that take `self` by value on a `deref` variant](#methods-that-take-self-by-value-on-a-deref-variant)
- [Variants and methods under `#[cfg]`](#variants-and-methods-under-cfg)
- [Where the block goes](#where-the-block-goes)
- [On a struct](#on-a-struct)

## Choose the variants

The attribute lists the variants a method is forwarded to:

- **Some variants by name**, `#[dispatch(Circle)]` or
  `#[dispatch(Circle, Square)]`. The method needs a body, the fallback
  for the variants you did not list.
- **Every variant**, `#[dispatch(*)]`. The method has no body: end the
  signature with `;`. (A body under `*` means something else: see
  [Forward to every variant but a
  few](#forward-to-every-variant-but-a-few).)
- **Every variant, one by one**, `#[dispatch(Circle, Square)]` on an enum
  with only those two. This is the same as `*`, so the method has no body
  either. No variant is left for a body to serve.

## Forward to every variant but a few

Under `*`, a method may still have a body. The body is then one
`match self` that holds the arms the enum handles itself, and every
variant those arms do not name is forwarded. Use it to forward to
every variant except a few, such as a unit variant beside the payloads:

```rust
#[whichever::dispatch]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
    Empty,
}

#[whichever::dispatch]
impl AnyShape {
    #[dispatch(*)]
    pub fn describe(&self) -> String {
        match self {
            Self::Empty => String::from("nothing"),
        }
    }
}
```

`Circle` and `Square` are forwarded; `Empty` returns `"nothing"`.

The generated `match` holds the forwarding arms first, in the enum's
order, then your arms as you wrote them. It has no wildcard arm, and that
is what keeps it safe:

- A variant you add to the enum later is forwarded, with no second edit.
- A variant without a payload and without an arm of yours gives an error
  on the method, because there is nothing to forward to.

Your arms follow the same rules as in a
[partial impl block](partial-impl-blocks.md#forward-to-every-variant-but-a-few).
Among them:

- They name their variants by path, as `Self::Empty`.
- They are written in the method's declared return type. No `with`
  wraps them.
- They may read the fields their pattern binds.

One difference from a partial impl block: a `Self` argument is no
obstacle here, because an inherent method hands it on as it is (see
[`Self` arguments](#self-arguments)).

## How the call is written

Each arm calls the payload's method with method-call syntax,
`inner.describe()`. So autoref, autoderef, and the rule that an inherent
method wins over a trait method of the same name all work as in
hand-written code.

On a [`deref` variant](routes.md), the call goes to what the payload
points at: `(**inner).describe()`. A method that the pointer itself has
under the same name never takes over. `Box` has an `as_ref`, for
example, but a forwarded `as_ref` calls the one on the boxed value.

## Options

Options follow the variants in the attribute, as in
`#[dispatch(Circle, with = Some)]`.

### `with = path`: wrap the result

`with = path` passes each forwarded result through `path` on its way out:

```rust
/// Only circles have a radius.
#[dispatch(Circle, with = Some)]
pub fn radius(&self) -> Option<f64> {
    None
}
```

The `Circle` arm becomes `Some(inner.radius())`; every other variant
gets the body, `None`.

- `path` may be a generic function. It is called separately in each arm,
  so payloads that return different types can meet in one declared
  return type.
- `with = Into::into` rebuilds the enum through its `From` impls. Use it
  to return the enum at another lifetime, as in
  `fn into_static(self) -> PlayableId<'static>`.

### `call = name`: forward to a method with another name

```rust
#[dispatch(*, call = describe)]
pub fn label(&self) -> String;
```

Each arm calls the payload's `describe`, so the enum's `label` returns
what the payload's `describe` returns.

### `field = name`: read a field

`field` and `value` make the arm a place instead of a call. `field = name`
reads that field of the [target](glossary.md#target):

```rust
#[dispatch(*, field = line_no)]
pub fn line_no(&self) -> usize;
```

Every arm is `inner.line_no`. The field is borrowed when the method
returns a reference, and read by value otherwise.

### `value`: return the target itself

`value` makes the arm the target itself, for `with` to wrap or for the
return type to widen. Like a field, it is borrowed where the method
returns a reference and read where it returns a value:

```rust
#[dispatch(Controller, Spawn, value, with = Some)]
fn as_owned(&self) -> Option<&dyn Owned> {
    None
}

#[dispatch(*, value)]
fn handle(&self) -> Handle;
```

- `as_owned` is `Some(inner)` for `Controller` and `Spawn`, and `None`
  for the rest.
- `handle` is `*inner` in every arm. This works the same for a payload
  and for a [marked field](glossary.md#mark).
- `via = .name, value` is `field = name` spelled out.

### How `field` and `value` read the return type

- Under `with = Some` or `with = Ok`, the borrow is read inside the
  `Option` or the `Result`. So `-> Option<&T>` with `field = name` gives
  `Some(&inner.name)`.
- Any other `with` is handed the target as it stands. Under `&self`, that
  is the reference the pattern binds. For a place reached through `via`
  or `field`, it is the place by value.
- A method returning a raw pointer is also handed the target as it
  stands. So is a method returning an `impl Trait` on a dispatch
  struct.
- The return type is read as written, so a type alias for a reference
  reads as a value.

### `via = route`: reach the target another way

`via = route` reaches the target for this method alone. The
[route](routes.md) starts at the payload and wins over the variant's own
route, if the variant has one:

```rust
#[dispatch(Held, via = .boxed*, call = describe)]
pub fn held_description(&self) -> String {
    String::new()
}
```

## Methods that return `Self`

A method returning `Self`, `Option<Self>` or `Result<Self, E>` wraps the
payload's result back into the variant, as a trait's forwarding impl
does.

With `with`, the payload returns the plain value and `with` lifts the
wrapped variant:

```rust
#[dispatch(Circle, with = Some)]
pub fn grown(&self) -> Option<Self> {
    None
}
```

This calls a `Circle::grown` that returns a `Circle`. The arm wraps it
in `Self::Circle`, and `with` wraps that in `Some`.

## `async` methods

An `async` method, or one that returns `impl Future`, is awaited in each
arm:

```rust
#[dispatch(*)]
pub async fn fetch(&self) -> u32;
```

## `Self` arguments

A `Self` argument is the enum itself. It is handed to the payload's
method as it is, so the payload's method takes the enum:

```rust
impl Circle {
    pub fn same_size(&self, other: &AnyShape) -> bool { /* ... */ }
}

#[whichever::dispatch]
impl AnyShape {
    #[dispatch(*)]
    pub fn same_size(&self, other: &Self) -> bool;
}
```

On a struct this is different; see [On a struct](#on-a-struct).

## Methods that take `self` by value on a `deref` variant

A value cannot be moved out of a `deref` variant that holds anything but
a `Box`. You cannot move a value out of an `Rc` or out from behind a
reference. So on such a variant, `field` or `value` under `self` or
`self: Box<Self>` gives an error.

The exception is a return type that borrows the place, because then
nothing moves. Over `Borrowed(&'a Name)`, this hands over the reference
the variant holds:

```rust
#[whichever::dispatch]
pub enum Lent<'a> {
    #[dispatch(deref)]
    Borrowed(&'a Name),
    #[dispatch(deref)]
    Other(&'a Name),
}

#[whichever::dispatch]
impl<'a> Lent<'a> {
    #[dispatch(*, value)]
    pub fn name(self) -> &'a Name;
}
```

A method forwarded as a call (no `field` or `value`) under `self` is the
arm you would write by hand, `(*inner).base()`. The payload's method
decides what moves:

```rust
#[whichever::dispatch]
pub enum Node<'a> {
    #[dispatch(deref)]
    Int(&'a Int),
    #[dispatch(deref)]
    Float(&'a Float),
}

#[whichever::dispatch]
impl<'a> Node<'a> {
    #[dispatch(*)]
    pub fn base(self) -> &'a Base;
}
```

If `Int::base` takes `&self`, the arm borrows the `Int`. If it takes
`self`, you get rustc's error, as you would in a hand-written arm.

## Variants and methods under `#[cfg]`

- A variant under `#[cfg]` may be listed. In a build where the variant is
  compiled out, the method uses its body.
- A method that cannot be forwarded gives its error under the method's
  own `#[cfg]`. A build that compiles the method out gives no error,
  since it would have no hand-written method there either.

## Where the block goes

The `impl` block has to be in the enum's crate, which inherent impls are
anyway.

The enum needs `#[whichever::dispatch]` on it. A bare `#[dispatch]`, with
no trait, registers the enum for inherent blocks without forwarding any
trait.

## On a struct

An inherent block on a [dispatch struct](structs.md) forwards the same
way:

```rust
#[whichever::dispatch]
pub struct Pair {
    #[dispatch]
    left: Circle,
    right: Circle,
}

#[whichever::dispatch]
impl Pair {
    #[dispatch(*)]
    pub fn radius(&self) -> u32;

    #[dispatch(right, call = radius)]
    pub fn right_radius(&self) -> u32;
}
```

- `*` is the field marked with a bare `#[dispatch]`, or the only field.
- A field's name is that field, marked or not.
- A method lists one field. Every value of the struct holds all its
  fields, so the `match` has one arm and there is nothing to choose
  between. Listing two fields gives an error; write a method that needs
  both by hand.
- A `Self` argument is the other value's field, as in a struct's trait
  impl.
- A return-position `impl Trait` is forwarded as it is.
