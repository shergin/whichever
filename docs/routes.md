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
forwards to. Both places also take `via_mut = route`, a second route for
the methods that borrow the target mutably; see
[A route for a mutable borrow](#a-route-for-a-mutable-borrow).

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
field can take a route of its own, `deref`, `via = route` or
`via_mut = route`, and no other option. A route for a mutable borrow is
given by the variant or by its marked field, not by both: two would
leave you guessing which one wins, so you get an error.

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
`via_mut(Pair = route)` gives that trait its own route for a mutable
borrow the same way. A partial impl block takes both on its own
attribute. (After a trait, a route is always keyed by the variant or the
field it goes through, so `via_mut = route` there is an error: write
`via_mut(Pair = route)`.)

### On an inherent method

An [inherent method](inherent-methods.md) can take a route for that one
method:

```rust
#[dispatch(*, via = .content(), call = refresh)]
pub fn refresh(&mut self);
```

Here the first `*` means "every variant", as usual in an inherent
method's attribute. The route is `.content()`.

A route on a method's own mark is that method's alone, and the method
follows it whatever it borrows: `&self`, `&mut self` or `self`. The same
holds for a method you mark in a
[partial impl block](partial-impl-blocks.md). So a mark takes `via`, and
`via_mut` on a mark is an error: the method's receiver is right beside
it.

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

## A route for a mutable borrow

A route that ends in a method call hands over what the method returns,
as it is. So one method serves one way of borrowing:

- a getter that lends the target, `fn source(&self) -> &Source`, serves
  `&self`;
- a getter that lends it mutably, `fn source_mut(&mut self) -> &mut
  Source`, serves `&mut self`;
- a method that returns the target by value serves `self`.

A type with a getter pair therefore needs two routes. Write the second
with `via_mut`:

```rust
#[whichever::dispatch(AudioRead)]
pub struct RegisteredAudio {
    #[dispatch(via = .source(), via_mut = .source_mut())]
    warp: Warp,
}
```

`position(&self)` is now `self.warp.source().position()`, and
`read(&mut self, ..)` is `self.warp.source_mut().read(..)`. The getter
runs as it would in your own code, so a `source_mut` that also clears a
cache still clears it. A route to the field would skip that.

Who follows which route:

- **A method that borrows the target mutably follows `via_mut`.** That
  is a method taking `&mut self` or `self: Pin<&mut Self>`, and a
  `&mut Self` argument.
- **Every other method follows `via`.**
- **Each `Self` argument takes the route of its own borrow.** So
  `fn absorb(&mut self, other: &Self)` is
  `absorb(inner.source_mut(), other.source())`.

### The fine print

- **Without `via_mut`, a mutable borrow through a route that ends in a
  method call is an error on the variant.** The method may lend shared
  only, and rustc would then report every such method. The error names
  `via_mut`, and says whether `&mut self`, `self: Pin<&mut Self>` or a
  `&mut Self` argument is the cause. A route that ends in a field is
  fine: a field is a place, borrowed either way.
- **A trait's `via(Variant = ..)` replaces both of the variant's own
  routes.** So a mutable borrow never follows the variant's `via_mut` to
  another target than the trait's route reaches. Where the trait's route
  ends in a method call, give the trait its own `via_mut(Variant = ..)`
  too, or you get the error above.
- **A trait's `via_mut(Variant = ..)` given alone replaces the variant's
  `via_mut` only.** The variant's own `via` still serves the other
  methods.
- **A method's own route** (see
  [On an inherent method](#on-an-inherent-method)) is followed whatever
  the method borrows. `#[dispatch(*, via = .data_mut(), call = to_send)]`
  on `fn to_send(&mut self, version: u16) -> Vec<u8>;` is the arm you
  would write, `inner.data_mut().to_send(version)`.
- **An inherent method's `value` through a route that ends in a method**
  hands over the method's result as it is: `inner.data()` under `&self`,
  `inner.data_mut()` under `&mut self`.

## What a route can and cannot do

- **A route that ends in a method call hands over the method's result.**
  Use it for the receivers that result fits, and a second route,
  `via_mut`, for a mutable borrow, as
  [above](#a-route-for-a-mutable-borrow).
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
