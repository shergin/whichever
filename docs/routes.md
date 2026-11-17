# Reaching the target

By default, each arm calls the trait method on the
[payload](glossary.md#payload), the value the variant holds. Sometimes
that is not the value you want. The payload may be a `Box` around the
real thing, or a struct whose field implements the trait. A
[route](glossary.md#route) says: start from the payload, take these
steps, and call the trait method on what you reach. What you reach is the
[target](glossary.md#target).

```rust
pub struct Account {
    pub base: Square,
    pub id: u32,
}

#[whichever::dispatch(Shape)]
pub enum AnyShape {
    #[dispatch(via = .base)]
    Account(Account),
    Plain(Square),
}
```

`Account` does not implement `Shape`, but its field `base` does. The
`Account` arm calls `area` on `&inner.base`, and the `Plain` arm calls it
on the payload, as usual.

## The steps

A route is a sequence of steps. There are three kinds:

| Step | Reaches |
| --- | --- |
| `*` | what the payload dereferences to; `#[dispatch(deref)]` is another way to write this route |
| `.field` or `.0` | a field, by name or by position |
| `.method()` | what a method returns; the method takes no arguments |

You can chain them in any order: `via = .account.base` (a field of a
field), `via = .boxed*` (what a field points at), `via = *.base` (a field
of what the payload points at), `via = .inner.base()` (what a field's
method returns).

`deref` already is the route `*`, so a variant takes either `deref` or
`via = ..`, not both.

## Where you write a route

You write a route in one of four places. These are the places where you
would write the same thing by hand.

### On a variant or a struct field

The route applies to every trait the enum forwards:

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    #[dispatch(via = .base_account)]
    Module(Module),
    #[dispatch(via = .1)]
    VarChar((Label, Square)),
}
```

On a [struct](structs.md), the same option goes on the field the struct
forwards to.

### On one field of a variant with several

When a variant has several fields, mark the one to forward to with
`#[dispatch]`, the same marker a struct uses:

```rust
use std::fs::File;
use std::io::LineWriter;
use whichever::foreign::Write;

#[whichever::dispatch(Write)]
pub enum Console {
    ConOut(#[dispatch] LineWriter<File>, u32),
    Node { id: u32, #[dispatch] data: Data },
}
```

The arm's pattern skips the other fields with `_` or `..`. The marked
field can take a route of its own, `deref` or `via = route`, and no other
option.

### On the enum's attribute, for one trait

When the route through one variant differs from trait to trait, write it
on the enum's attribute for each trait, with the variant's name:

```rust
#[whichever::dispatch(Area, via(Pair = .1))]
#[whichever::dispatch(Named, via(Pair = .0))]
pub enum Any {
    Pair((Label, Square)),
    Plain(Square),
}
```

For that trait, this route wins over a route on the variant itself.

### On an inherent method

An [inherent method](inherent-methods.md) can take a route for that one
method:

```rust
#[dispatch(*, via = .content(), call = refresh)]
pub fn refresh(&mut self);
```

Here the first `*` means "every variant", as usual in an inherent
method's attribute. The route is `.content()`.

## What the arm looks like

The arm stays one line, and it borrows the target as the receiver asks:
`&inner.base` for `&self`, `&mut inner.base` for `&mut self`.

When the attribute can name the target's type, the call names it. That is
the case for a marked field, `<Square as Shape>::area(inner)`, and for a
`deref` route, `<<Box<Square> as Deref>::Target as Shape>::area(&**inner)`.

The type of a field or of a method's result cannot be named from inside
the macro. Then the call is qualified by the trait alone:

```rust
AnyShape::Account(inner) => <_ as Shape>::area(&inner.base),
```

It is still a fully qualified call, so an inherent method of the same
name on the target still never takes over.

## What a route can and cannot do

- **A route that ends in a method call serves `&self` or `self`, not
  both.** The method's result is handed over as it is. A method that
  lends the target, `fn base(&self) -> &Square`, serves the trait's
  `&self` methods. A method that returns the target by value serves its
  `self` methods. Use the route for the receivers its method fits.
- **A route that ends in a method call cannot serve `&mut self`.** You
  cannot borrow mutably through a call's result, so you get an error. The
  same goes for `self: Pin<&mut Self>` and a `&mut Self` argument, and the
  error says which. Route to a field instead, or write the method by hand.
- **`deref` over an `Rc`, an `Arc` or a `&T` cannot lend mutably**, because
  those only lend what they point at as shared. You get an error on the
  payload for:
  - a trait method taking `&mut self` or `self: Pin<&mut Self>`;
  - a trait method with a `&mut Self` argument;
  - an inherent method that returns a place as `&mut`.

  A shared borrow still works, such as `-> &Name` under `&mut self`. To
  lend mutably, hold a `Box` or a `&mut`.
- **A method returning `Self` cannot use a routed variant**, `deref`
  included. The result is a new target, and the rest of the payload, the
  pointer or the other fields, is gone, so there is nothing to wrap it
  back into. The same goes for a variant with other fields next to the
  marked one.
- **A variant whose one field is named and marked** is the exception:
  `Named { #[dispatch] shape: Square }` is rebuilt with its braces, so
  `-> Self` works. But `Option<Self>` and `Result<Self, E>` are rebuilt
  with a path, and no path builds such a variant, so those are an error.
  Write it as a tuple variant, `Named(Square)`, instead.
- **No `From` or `TryFrom`** for a variant with other fields next to the
  payload, or with the payload in a named field. (For other fields, a
  conversion from the payload alone would not know what to put in them.)
- **No agreement check** for a variant whose target has no nameable
  type, such as one reached through a field or a method. The checks need
  the type. So a disagreeing associated type shows up as a type error in
  the arm of a method that returns it. A disagreeing associated const
  shows up nowhere, since the enum takes its consts from another
  variant: the first one whose target does have a nameable type.

## Holding an open set

A `deref` variant can hold a trait object, `Box<dyn Trait>` or
`&dyn Trait`. This is how an enum holds an open set next to its sized
payloads: the listed types stay inline, and any other implementor fits
in the trait object.

```rust
#[whichever::dispatch]
pub enum AnyShape {
    Plain(Square),
    #[dispatch(deref)]
    Boxed(Box<dyn Shape>),
}
```

The enum lists no trait here, because the partial impl block below is its
impl of `Shape`.

A method the trait declares with `where Self: Sized` cannot be called on
a trait object, so you get a one-sentence error for it. Leave that method
to the trait's default in a [partial impl block](partial-impl-blocks.md),
as you would by hand, and the rest is forwarded:

```rust
#[whichever::dispatch]
impl Shape for AnyShape {
    #[dispatch(default)]
    fn doubled(&self) -> u32
    where
        Self: Sized;
}
```

Some pointers implement the trait themselves: `Box<dyn Iterator>` is an
`Iterator`. Hold such a pointer without `deref`. Then every method,
`Self: Sized` or not, is forwarded to the pointer itself, and you need no
partial impl block.
