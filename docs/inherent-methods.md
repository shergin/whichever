# Inherent methods

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
resolution apply as in hand-written code. On a `deref` variant the call
goes to what the payload points at, `(**inner).describe()`, so a method the
pointer has of the same name, such as `Box`'s `as_ref`, never takes over.
`with = path` wraps each forwarded
result in a call to `path`, which may be a generic function, called
separately in each arm, so payloads whose results differ can meet in one
declared return type, and `with = Into::into` rebuilds the enum through its
`From` impls, which is the way to return the enum at another lifetime,
`fn into_static(self) -> PlayableId<'static>`; `call = name` forwards to a
method of another name;
`async` methods and methods returning `impl Future` are awaited; and
methods without the attribute are left alone. A method returning `Self`,
`Option<Self>` or `Result<Self, E>` wraps the payload's result back into
the variant, as trait dispatch does. With `with`, the payload returns the
plain value and `with` lifts the wrapped variant: `#[dispatch(Circle, with =
Some)]` on `fn grown(&self) -> Option<Self>` calls a `Circle::grown` that
returns a `Circle`. Two options make the arm a place instead of a call:
`field = name` reads that field of the target, by reference when the
method returns one and by value otherwise, so `#[dispatch(*, field =
line_no)]` on `fn line_no(&self) -> usize;` is `inner.line_no` in every
arm; and `value` makes the arm the target itself, which `with` wraps or
the return type widens, borrowed where the method returns a reference and
read where it returns a value, as a field is. So `#[dispatch(Controller,
Spawn, value, with = Some)]` on `fn as_owned(&self) -> Option<&dyn Owned>
{ None }` is `Some(inner)` for the named variants, `#[dispatch(*, value)]`
on `fn handle(&self) -> Handle;` is `*inner` in every arm, a payload or a
marked field alike, and `via = .name, value` is `field = name` spelled
out. Under `with = Some` or `with = Ok` the borrow is read inside the
`Option` or the `Result`, so `-> Option<&T>` gives `Some(&inner.name)`.
Any other lift is handed the target as it stands, the reference the
pattern binds under `&self` or a place reached through `via` or `field`
by value, and so is a method returning a raw pointer or, on a dispatch
struct, an `impl Trait`. The return type is read as written, so a type
alias for a reference reads as a value. `via = route` reaches the target
for this method alone, over the variant's own route. A value cannot be
moved out of a `deref` variant holding anything but a `Box`, so a method
taking `self` is reported there, and so is a place under `self:
Box<Self>` unless the return type borrows it. A `Self` argument is
the enum itself and is handed to
the payload's method as it is. A variant under `#[cfg]` may be named; where
it is compiled out, the method takes its body. The `impl` block has to be
in the enum's crate, which inherent impls are anyway; a bare `#[dispatch]`
on an enum registers it for this without dispatching a trait. A block on a
dispatch struct forwards the same way: `*` is the field marked bare, or the
only field, and a field's name is that field, marked or not; there a `Self`
argument is the other value's field, as in trait dispatch, and a
return-position `impl Trait` is forwarded as it is.
