# Options

All options follow the trait path in `#[dispatch(..)]`.

- `Name = Type` fixes the associated type `Name` instead of taking it from
  the first variant. A method returning `Self::Name` is converted with
  `Into`, one returning `Result<_, Self::Name>` with `map_err(Into::into)`,
  and `Option<Self::Name>`, `Result<Self::Name, E>`, `Result<Self,
  Self::Name>` and a tuple holding `Self::Name` are mapped the same way, so
  variants with their own error types can share one enum:

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

  An expression on the trait method or on the enum may use the method's
  parameters, a `Self` argument included, and `self` when the method
  takes `&self`; `&mut self` and `self` are moved into the `match`.
  A path in an expression on the trait method resolves where the enum is,
  as the signature types do, so a helper beside the trait is spelled from
  `crate::`.

- `from = false` suppresses the `From<Payload>` impls. A payload type held
  by two variants gets none either way, and a payload whose `From` is
  written by hand, or that a derive supplies, stays out with
  `#[dispatch(from = false)]` on its variant, since a second impl of the
  same `From` is a conflict.
- `try_from = true` adds `TryFrom<Enum> for Payload` for every payload type,
  with the enum as the error, so a mismatch hands the value back; `.unwrap()`
  on the result therefore needs the enum to be `Debug`. `try_from = ref`
  adds `TryFrom<&Enum> for &Payload` to that, and `try_from = ref_mut` adds
  `TryFrom<&mut Enum> for &mut Payload` too, each handing the reference
  back as the error; the words are `derive_more`'s.
- `into = Target` adds `From<Enum> for Target`, with each arm converting
  the payload through its own `Into<Target>`, so an enum over messages
  that all convert into one wire type converts in one line; the option
  may be repeated. Every variant has to hold a payload and take part in the
  conversions.

Four options go on a variant. `#[dispatch(from = false)]` leaves the
variant out of the `From` and `TryFrom` impls, for a payload such as
`String` that should not convert into the enum, and
`#[dispatch(try_from = true)]` adds the `TryFrom` impl for that payload
alone. `#[dispatch(deref)]` calls the trait on what the payload
dereferences to, so `Box<T>`, `Rc<T>`, `Arc<T>`, `&T` and `&mut T`
payloads dispatch like a plain `T`, and `#[dispatch(via = route)]` calls
it on a field of the payload or on what a method of it returns; see
[Reaching the target](routes.md):

```rust
#[whichever::dispatch(Shape)]
pub enum AnyShape<'a> {
    #[dispatch(deref)]
    Boxed(Box<Circle>),
    #[dispatch(deref)]
    Borrowed(&'a Square),
    #[dispatch(via = .shape)]
    Labelled(Labelled),       // struct Labelled { shape: Square, label: String }
    Plain(Square),
}
```

A trait's own route through a variant, `via(Variant = route, ..)` after
the trait path, wins over the variant's for that trait; a variant with
several fields marks the one to forward to with `#[dispatch]` and is
otherwise reported. Several traits are dispatched by repeating the
attribute; a supertrait is one more attribute:

```rust
#[whichever::dispatch(Shape)]
#[whichever::dispatch(Drawable)]
pub enum AnyShape { /* .. */ }
```

A generic trait is dispatched at an instantiation named on the enum: a
concrete one, `#[whichever::dispatch(Encoder<Json>)]`, or the enum's own
parameters, `#[whichever::dispatch(GridTrait<T>)]` on `enum Grid<T>`; a
parameter with a default may be left out, `#[whichever::dispatch(Enc)]`
for `trait Enc<F = Json>`, as `impl Enc for A` leaves it out. Type,
lifetime and const parameters are substituted in every signature, and a
projection `F::Value` becomes `<Json as Format>::Value` when `F` has one
bound; with several bounds the trait has to spell `<F as Format>::Value`
itself. A payload that is a type parameter of the enum, `Raw(Raw)` on
`enum Lazy<Raw, Parsed>`, or `Box<T>` under `deref`, gives the impl the
bound `Raw: Trait`, which is the bound a hand writes; a struct's field the
same, `impl<T: Shape> Shape for Wrapper<T>` for `struct Wrapper<T>(T)`.
