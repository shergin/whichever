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
forwarding's arms written out, `*` with a body is the arms the enum answers
itself, below, and `#[dispatch(default)]` on a bodiless method
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

When the enum answers only a few variants itself, `#[dispatch(*)]` with a
body writes the list the other way round. The body is one `match self`
holding the arms the enum answers itself, and every variant those arms do
not name is forwarded; the method is the `match` a hand writes, without the
arms that only forward:

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

The generated `match` is a forwarding arm for each other variant, in the
enum's order, then these arms as written, with no wildcard, so a variant
added to the enum later is forwarded with no second edit, and one that
holds no payload is reported on the method, naming the arm to write. An
arm names its variants by path, `Self::Unknown(..)`, through a guard, an
or-pattern, a reference or a binding `name @ ..`, and a variant it names
is not forwarded, guard or not; an arm that names none, `_` or a
binding, is reported, since the form with a wildcard is the named list
with a body. The arms are the method's own, written as they stand in the
declared return type, and read the fields their pattern binds; in an
`async fn` they sit where the forwarding arms sit, inside the future. An
arm under `#[cfg]` answers where the `cfg` holds, and its variant is
forwarded where it does not. A trait method with a `Self` argument, which
is matched beside `self`, and a method over an enum whose every variant is
under `#[cfg]`, which is matched as `match *self`, hold no arms the enum
answers itself; the named list with a body writes both.

That is how a trait reaches a unit variant, a variant whose payload has no
impl, or a variant the enum answers itself whether it holds a payload or
not: the method names the rest, or under `*` its body holds the arms of those
variants. A mark under `#[cfg]` follows the rule of a method written under
one, `#[dispatch(default)]` included: the method is left to the default,
or forwarded as marked, where the `cfg` holds; where it does not, it is
forwarded as a mark of the same name without a `#[cfg]` says, or else to
every variant. A variant no method can forward to, such as a unit variant,
is reported in the builds in which some method is forwarded to every
variant and in no other, so a mark under the trait method's own `#[cfg]`,
which leaves the forwarding no build that has the method, reports nothing.
A method written or marked in the block is yours, and so are its
attributes: it carries what you write on it, with `#[inline]` added where
you write no `#[inline(..)]`, and nothing of the trait method's own, as a
hand-written impl inherits nothing from the trait's declaration; only the
methods the block leaves to the plain forwarding, whose signatures the
attribute writes, carry the trait method's `#[inline(..)]` and `#[allow]`.
The block's own header,
generics and bounds included, becomes the impl's header, and may name the enum's parameters as it likes, `impl<T, Ty> Get<T>
for ImmOrNode<Ty>`. The arms name the payload types as the enum spells
them, so a block in another module needs them in scope there; the converse
holds too, so when the trait's signature types are not in scope where the
enum is, an empty block in a module where they are, such as one holding
`use trait_crate::*;`, is the place to write the impl. Outer
attributes on the block are carried onto the impl: `#[typetag::serde(name =
"..")]` goes there, with `#[whichever::dispatchable]` above
`#[typetag::serde]` on the trait, so that typetag's own methods are the
block's and never forwarded. A macro invocation in the block is reported,
since the items it writes cannot be told from the ones to forward; write
them out in the block. One under `#[cfg]` is reported only where the `cfg`
holds: where it does not, the invocation is not in the block, as it is not
in a hand's, and the block is read without it, so a method the macro
would have written is forwarded. A method written in the block that still
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
