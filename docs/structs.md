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
takes every trait not named elsewhere; the struct's attribute still names
every trait it dispatches. A field names a trait as the struct's attribute
spells it, arguments included, so two instantiations of one generic trait,
`Encoder<Json>` and `Encoder<Xml>`, or two traits of one name from two
modules, go to two fields:

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

`#[dispatch(deref)]` on the field reaches through a `Box` or another
pointer. A method returning `Self` is rebuilt only around a struct with no
other field; otherwise it is an error, and the method goes in a partial impl
block, which works on a struct like on an enum. A struct's `match` has one
arm, so it forwards a return-position `impl Trait` as it is, which an enum
cannot. A struct gets no `From` impls. `Self` arguments need no `mismatch`:
two values of the struct always hold the same field, and the argument is
unwrapped to it.
