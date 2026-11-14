# Methods the enum implements itself

A trait method without a receiver, such as a constructor, has no variant to
dispatch on, and sometimes the enum's answer to a method differs from every
payload's. Put `#[dispatch]` on a trait impl block holding just those
items, and the rest of the trait is forwarded around them:

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

The enum then does not name that trait in its own attribute, since the
block is the impl; naming it in both is reported on the block. Options for
the trait, `Name = Type` and `mismatch`, go on the block's attribute; an
associated type written in the block counts as an override too. A generic
associated type is declared here, since no first variant can supply one,
`type Item<'a> = AnyItem<'a> where Self: 'a;`, and each payload's own
converts into it through `Into`, as for any override. A method written in
the block replaces the forwarding for every variant, provided methods
included, which is the way to leave a payload's method out on purpose; one
written under `#[cfg]` replaces it where the `cfg` holds and is forwarded
where it does not, as a hand writes the two arms. A method in the block
may name the variants it forwards to, as an inherent method does, and its
body is then the arm for every other variant, two different variants of a
`Self` argument included; `*` with a bodiless method is the plain
forwarding written out, and `#[dispatch(default)]` on a bodiless method
leaves it to the trait's default, neither forwarded nor written; over a
restatement that is the real trait's default, which the restatement cannot
see:

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

That is how a trait reaches a unit variant, a variant whose payload has no
impl, or a variant the enum answers differently: the method names the
rest. A mark under `#[cfg]` follows the rule of a method written under
one, `#[dispatch(default)]` included: the method is left to the default,
or forwarded as marked, where the `cfg` holds; where it does not, it is
forwarded as a mark of the same name without a `#[cfg]` says, or else to
every variant.
The block's own header,
generics and bounds included, becomes the impl's header, and may name the enum's parameters as it likes, `impl<T, Ty> Get<T>
for ImmOrNode<Ty>`. The arms name the payload types as the enum spells
them, so a block in another module needs them in scope there; the converse
holds too, so when the trait's signature types are not in scope where the
enum is, an empty block in a module where they are is the place to write
the impl. Outer
attributes on the block are carried onto the impl: `#[typetag::serde(name =
"..")]` goes there, with `#[whichever::dispatchable]` above
`#[typetag::serde]` on the trait, so that typetag's own methods are the
block's and never forwarded. A method written in the block that still
needs the payloads' answer calls an inherent `#[dispatch(*, call = name)]`
method beside it. Like inherent blocks, it has to be in the enum's crate.

## The impl header

The block's header is the impl's header, so an empty block is how the impl
gets a header the enum's attribute cannot write: generics the enum does not
have, bounds, or one instantiation. A trait generic over its own parameter
is implemented for every `F`:

```rust
#[whichever::dispatch]
impl<F: Format> Encoder<F> for AnyEncoder {}
```

A trait with a lifetime the enum lacks names it, `impl<'r> Renderable<'r>
for Node {}`; a trait implemented for one instantiation of a generic enum
names that, `impl ColNames for Lazy<Raw<'static>, Parsed> {}`; and a bound
on a type parameter that the trait's own `Param: Trait` bound does not cover
goes in the header, `impl<T: Shape + Clone> Shape for Wrapper<T> {}`.
