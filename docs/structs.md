# Forwarding from a struct field

A dispatch enum is often private, behind a public struct that forwards the
same trait by hand, and the hand-written half forwards only the required
methods, so a payload's override of a provided method never reaches the
public type. `#[dispatch]` on the struct forwards to its one field, or to
the field marked `#[dispatch]`, every method included:

```rust
#[whichever::dispatch(FlowCtrlHooks)]
enum FlowCtrlInner {
    Window(WindowFlowCtrl),
    XonXoff(XonXoffFlowCtrl),
}

#[whichever::dispatch(FlowCtrlHooks)]
pub struct StreamFlowCtrl {
    #[dispatch]
    inner: FlowCtrlInner,
    hooks: Hooks,
}
```

The field may be any implementor, not only a dispatch enum, so the same
attribute replaces a field-delegation derive:

```rust
#[whichever::dispatch(Addresses)]
pub struct Beacon {
    #[dispatch]
    header: ManagementHeader,
    body: Vec<u8>,
}
```

A field may name the traits it is the target of, and the field marked bare
takes every trait not named elsewhere. A trait's impl has one home: the
struct's attribute, which names every trait whose impl it writes, or a
[partial impl block](partial-impl-blocks.md), and a trait goes to the field
that names it whichever home writes the impl. A field names a trait as its
home spells it, the attribute or the block's header, arguments included,
so two instantiations of one generic trait, `Encoder<Json>` and
`Encoder<Xml>`, or two traits of one name from two modules, go to two
fields:

```rust
#[whichever::dispatch(Rom)]
#[whichever::dispatch(Interrupt)]
#[whichever::dispatch(Named)]
pub struct Mapper {
    #[dispatch(Rom)]
    rom: Cartridge,
    #[dispatch(Interrupt)]
    signals: Signals,
    #[dispatch]
    label: Label,       // `Named`, and any other trait
}
```

A trait whose impl needs a header the attribute cannot write, such as a
bound that ties one field's type to another's, is a partial block's, and
the field still names it; the struct's attribute then leaves it out, since
the block is the impl:

```rust
#[whichever::dispatch(Named)]
pub struct Env<S, C, N> {
    pub syntax: S,
    #[dispatch(Catalog)]
    pub catalog: C,
    #[dispatch]
    pub named: N,
}

#[whichever::dispatch]
impl<S: Sorts, C: Catalog<Extractor = S::Extractor>, N> Catalog for Env<S, C, N> {}
```

A generic trait's block names its parameter, `impl<F, E: Encoder<F>, N>
Encoder<F> for Env<E, N> {}`, and the field names the trait as the header
does, `#[dispatch(Encoder<F>)]`. A method of the block marked `*` goes to
that field, and so does one that names it, `#[dispatch(catalog)]`; one that
names another field is reported, with the way to send the trait there,
which is to name it on that field. The struct does not see its blocks, so
a field's mark that no home claims forwards nothing and is not reported:
the trait's first use says that the impl is missing. One name spelled two
ways is reported, at either home: a home whose trait no field names as it
is spelled there, beside a field that names it with more or fewer path
segments, `Catalog` and `traits::Catalog`, or with or without its generic
arguments, `Encoder` and `Encoder<F>`, since the home would otherwise go
to the bare field.

`#[dispatch(deref)]` on the field reaches through a `Box` or another
pointer. A method returning `Self` is rebuilt only around a struct with no
other field; otherwise it is an error, and the method goes in a partial impl
block, which works on a struct like on an enum. `Option<Self>` and
`Result<Self, E>` are mapped with a path, `.map(Wrapper)`, which a tuple
struct has and a struct with a named field does not, so on `struct Named {
shape: Square }` they are an error too, which the tuple form
`struct Named(Square);` avoids; `-> Self` there is rebuilt by the braces,
as it is for a variant whose one field is named and marked. A struct's
`match` has one
arm, so it forwards a return-position `impl Trait` as it is, which an enum
cannot. A struct gets no `From` impls. `Self` arguments need no `mismatch`:
two values of the struct always hold the same field, and the argument is
unwrapped to it.
