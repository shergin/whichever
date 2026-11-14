# What is generated

For a trait and an enum over its implementors:

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

Three more things are in the expansion, and in `cargo expand`: the impl is
marked `#[automatically_derived]`, which tells Clippy and coverage tools
what it is; the bindings the arms use are named `__inner`, `__same0` and
the like, with `macro_rules`-style hygiene, so they never collide with a
name the trait wrote; and a `const _` block beside the impl holds the
agreement checks for the associated types and consts, which compile to
nothing. The enum keeps its definition in a crate-local carrier macro for
its impl blocks, re-exported under the enum's name.

## What each trait item becomes

| Trait item | Enum impl |
| --- | --- |
| `type X;` | `type X = <FirstVariant as Trait>::X;`, or the `X = Type` option. Every other variant is checked to agree; a disagreement is reported on that variant. |
| `const N: T;` | `const N: T = <FirstVariant as Trait>::N;`, or the `N = value` option. For an integer, a float, `bool`, `char` or `&str` const on a non-generic enum, every other variant is checked to agree at compile time, with the error on that variant. |
| method with a receiver | A `match` over the variants calling `<Payload as Trait>::method(payload, args..)`, with `#[inline]` or the trait method's own `#[inline(..)]`. |
| generic method | Called with its type and const parameters spelled out when no argument fixes them. |
| `self: Box<Self>` | The enum is moved out of its box and a `deref` variant's own `Box<Payload>` is handed over; a plain variant is an error, since boxing it again would allocate. |
| `self: Pin<&Self>`, `self: Pin<&mut Self>` | The enum is unpinned, matched, and the payload pinned again, which needs every payload to be `Unpin`. |
| default body | Ignored: forwarded like any other method. |
| `async fn`, `unsafe fn` | Arms are awaited; calls are wrapped in `unsafe { }`. |
| `unsafe trait` | An `unsafe impl`. |
| `-> impl Future<Output = T>` | The `match` is written in one `async move` block and every arm is awaited, as for `async fn`; `Send` and any other bound are checked against that block. On a struct the one arm returns the field's future as it is, unless its output is rewrapped or converted. |
| `-> Pin<Box<dyn Future<Output = T> + ..>>`, as `#[async_trait]` writes it | Forwarded as it is, one type for every arm; when `T` is rewrapped or converted, every arm is awaited in one `async move` block that is boxed and pinned again with the `Box` the signature names, as the attribute itself writes. |
| other `-> impl Trait` | An error on an enum, whose arms would each return a different opaque type; forwarded as it is on a struct, whose one arm returns one. |
| argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`; different variants give the `mismatch` expression. |
| return `Self` | Wrapped back into the same variant; `Option<Self>` and `Result<Self, E>` are mapped, under any path, `io::Result<Self>` included; a tuple holding `Self`, or an overridden associated type, is taken apart and built again around its converted elements. `Self` anywhere else in a return type, or a tuple under an `Option` or a `Result`, is an error, since the rewrap would need a closure; a provided method of that kind is left to the trait's default with `#[dispatch(default)]`. |
| method without a receiver | Left to the trait's default body, or written in a `#[dispatch] impl Trait for Enum` block; an error when it has neither. |
| `#[cfg]` on a method or an associated item | Decided in the trait's crate: the carrier macro is written once per combination of the trait's distinct predicates, each under its own `cfg`, so the generated item exists exactly when the trait's does, in a build of the trait's crate. |
| `#[cfg]` on a variant | Carried onto the arm, and decided where the enum is. An associated type or const comes from the first variant without a `#[cfg]`. An enum whose every variant is under `#[cfg]` is matched as `match *self` with `ref` bindings where the receiver is a reference, so a build that leaves no variant still compiles, as a hand's does. |
| `#[deprecated]` on a variant | Its arms and its conversions are written under `#[allow(deprecated)]`; the deprecation reaches the code that names the variant. |
| `#[deprecated]` on a method or an associated item | Forwarded under `#[allow(deprecated)]`, as a hand writes it; the deprecation reaches the callers through the trait's item. |
| `#[allow(..)]` on a method or an associated item | Carried onto the generated item, which repeats the signature the lint was allowed on. `warn`, `deny` and `expect` stay the trait crate's, and so does an `#[allow]` another macro added. |
