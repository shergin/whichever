# Glossary

The words these pages use, roughly in the order you meet them in the
[tour][tutorial].

[Agreement check](#agreement-check) |
[Carrier macro](#carrier-macro) |
[Dispatch enum](#dispatch-enum) |
[Dispatch struct](#dispatch-struct) |
[Fallback](#fallback) |
[Forwarding](#forwarding) |
[Inherent block](#inherent-block) |
[Limit](#limit) |
[Mark](#mark) |
[Mismatch](#mismatch) |
[Override](#override) |
[Partial impl block](#partial-impl-block) |
[Payload](#payload) |
[Provided method](#provided-method) |
[Restatement](#restatement) |
[Route](#route) |
[Target](#target) |
[Variant](#variant)

## Forwarding

Passing a method call on to the value inside the enum. For each trait
method, the enum gets one `match` with one arm per variant, and each arm
calls the same method on what that variant holds. The impl made of these
`match`es is the forwarding impl. See [What is
generated][what-is-generated].

```rust
AnyShape::Circle(inner) => <Circle as Shape>::area(inner),
```

## Dispatch enum

An enum with `#[dispatch]` on it. Usually the attribute lists a trait,
`#[whichever::dispatch(Shape)]`, and the enum gets the forwarding impl of
that trait. A bare `#[whichever::dispatch]` lists no trait. It makes the
enum a dispatch enum all the same, for an enum whose forwarding is all
in [inherent blocks](#inherent-block) or
[partial impl blocks](#partial-impl-block). See the
[tour][tutorial].

You *dispatch* a trait on an enum by putting `#[dispatch(Trait)]` on it;
the impl that results *forwards* each call to the payload.

## Dispatch struct

A struct with `#[dispatch(Trait)]` on it, which forwards the trait to one
of its fields. See [Forwarding from a struct field][structs].

## Variant

One case of the enum, as in Rust: `Circle(Circle)` is a variant of
`AnyShape`. A variant that a trait is forwarded to holds exactly one
unnamed field, its [payload](#payload), or marks one of its fields with
`#[dispatch]`. See [Limits][limits].

## Payload

The value a variant holds: the `Circle` inside `Circle(Circle)`. The
enum gets a `From<Payload>` impl for each payload type, unless you turn
it off with `from = false`; a payload type held by two variants gets
none. See [Options][options].

## Target

The value the trait method is called on. Usually it is the payload
itself. With a [route](#route), it is something reached from the
payload, such as the `Square` inside a `Box<Square>`, or a field of the
payload. See [Reaching the target][routes].

## Route

The steps from the payload to the [target](#target). A step is `*` (what
the payload dereferences to), `.field` or `.0` (a field), or `.method()`
(what a method returns), in any sequence. `#[dispatch(deref)]` on a
variant is the route `*`, and `#[dispatch(via = .shape)]` reaches a
field. See [Reaching the target][routes].

```rust
#[dispatch(deref)]
Boxed(Box<Square>),
```

## Carrier macro

The hidden `macro_rules!` macro that `#[dispatchable]` leaves next to the
trait. It holds the trait's signatures and the attributes the generated
items carry, and nothing else: no default body, no doc comment, no bound.
It has the trait's name, so `use shapes::Shape;` imports both, and that
is how an enum in another crate reaches the trait. For a `pub` trait it
is exported at the crate root. See [Compatibility][compatibility].

## Provided method

A trait method with a default body, also called a default method. The
enum forwards it like any other method, so a payload's own override is
what runs, never hidden by the default. See [What is
generated][what-is-generated].

## Restatement

A copy of the signatures of a trait that another crate owns, marked
`remote`, so that it has a carrier macro: `#[whichever::dispatchable(remote
= std::io::Write)]` on a trait that lists the methods to forward. An
enum or a struct another crate defines can be restated the same way.
`whichever::foreign` ships restatements of common `std` traits. See
[Traits, enums and structs from other crates][restatements].

## Partial impl block

A trait impl block with `#[dispatch]` on it, where you write some of the
trait's items and the rest are forwarded. It is how the enum gets a
method without a receiver, such as a constructor, or a method it handles
differently from every payload. The block is the impl, so the enum does
not list that trait in its own attribute. See [Partial impl
blocks][partial-impl-blocks].

## Inherent block

An inherent `impl` block of a dispatch enum (or a dispatch struct) with
`#[dispatch]` on it. Each method marked `#[dispatch(..)]` is forwarded to
the variants it lists, or to every variant with `*`, by calling the
payload's method of the same name. See [Inherent
methods][inherent-methods].

```rust
#[whichever::dispatch]
impl AnyShape {
    #[dispatch(*)]
    pub fn describe(&self) -> String;
}
```

## Mark

`#[dispatch]` or `#[dispatch(..)]` written on something inside the item
the attribute is on:

- On a method of an [inherent block](#inherent-block) or a
  [partial impl block](#partial-impl-block), it says which variants the
  method is forwarded to. In a partial impl block, `#[dispatch(default)]`
  leaves the method to the trait's default instead.
- On a field of a variant or a struct, it picks the field to forward to:
  the marked field.

See [Partial impl
blocks][partial-impl-blocks-mark],
[Inherent methods][inherent-methods],
[Reaching the target][routes] and
[Forwarding from a struct field][structs].

## Fallback

The body of a method that is forwarded to only some variants. The other
variants run it. Inherent methods and methods of a
partial impl block can have one. See [Inherent
methods][inherent-methods].

```rust
#[dispatch(Circle, with = Some)]
pub fn radius(&self) -> Option<u32> {
    None // the fallback, for every variant but `Circle`
}
```

## Mismatch

When `self` and a `Self` argument hold different variants, so there is
no one payload method to call. It is also the name of the option that
says what to return then, on the trait method or on the enum. See
[Options][options].

```rust
#[dispatch(mismatch = false)]
fn congruent(&self, other: &Self) -> bool;
```

## Override

`Name = Type` or `Name = value` on the attribute, fixing an associated
type or const on the enum instead of taking it from the first variant.
For a type, what a method returns is converted from each payload's own
type with `Into`. An associated type written in a partial impl block
counts as an override too. See [Options][options].

```rust
#[whichever::dispatch(Storage, Error = StorageError)]
```

## Agreement check

The compile-time check that every variant has the same associated type
or const. Without an [override](#override), the enum takes each
associated item from its first variant, and the check gives an error on
any variant that differs. It compiles to nothing. Some consts are taken
without a check; see [What is generated][what-is-generated-consts].

## Limit

Something the attribute cannot forward and refuses, with a one-sentence
error on the trait item or variant at fault, naming the fix where there
is one. The rest of the enum still compiles, so you see the error for the
thing to fix, and no wall of follow-on errors. Every limit has
a line in [Limits][limits] and a compile-fail case in `tests/ui`.

<!-- The links are reference-style because src/docs.rs includes this page in the crate docs and gives each label a target there. -->
[compatibility]: compatibility.md
[inherent-methods]: inherent-methods.md
[limits]: limits.md
[options]: options.md
[partial-impl-blocks]: partial-impl-blocks.md
[partial-impl-blocks-mark]: partial-impl-blocks.md#mark-a-method-instead-of-writing-it
[restatements]: restatements.md
[routes]: routes.md
[structs]: structs.md
[tutorial]: tutorial.md
[what-is-generated]: what-is-generated.md
[what-is-generated-consts]: what-is-generated.md
