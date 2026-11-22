//! The axes of stage 1: trait dispatch through the enum's attribute.
//!
//! Each axis is declared here with its values and words; what a value
//! means is its descriptor, in the module of its kind: [`super::target`],
//! [`super::receiver`], [`super::returns`], [`super::argument`],
//! [`super::qualifier`], [`super::path`] and [`super::item`].

use crate::axis;

axis! {
    /// How the first variant reaches the trait's implementor, its payload
    /// `P0`: its target kind.
    pub enum Target {
        /// `A(P0)`: the payload is the target.
        Plain = "plain",
        /// `#[dispatch(deref)] A(Box<P0>)`.
        DerefBox = "deref-box",
        /// `#[dispatch(deref)] A(Rc<P0>)`.
        DerefRc = "deref-rc",
        /// `#[dispatch(deref)] A(Arc<P0>)`.
        DerefArc = "deref-arc",
        /// `#[dispatch(deref)] A(&'a P0)`.
        DerefShared = "deref-ref",
        /// `#[dispatch(deref)] A(&'a mut P0)`.
        DerefExclusive = "deref-mut",
        /// `#[dispatch(via = .payload)] A(W0)`, a field of a wrapper.
        ViaField = "via-field",
        /// `#[dispatch(via = .1)] A((u64, P0))`, an element of a tuple.
        ViaIndex = "via-index",
        /// `#[dispatch(via = .lend())] A(W0)`, what a method of a wrapper
        /// lends.
        ViaMethod = "via-method",
        /// `#[dispatch(via = .copied())] A(W0)`, what a method of a
        /// wrapper returns by value.
        ViaValueMethod = "via-method-value",
        /// `A(#[dispatch] P0, u64)`, the marked field of a tuple variant.
        MarkedTuple = "marked-tuple",
        /// `A { label: u64, #[dispatch] held: P0 }`, the marked field of a
        /// struct variant.
        MarkedStruct = "marked-struct",
        /// `A { label: u64, #[dispatch(via = .payload)] held: W0 }`, a
        /// marked field with a route of its own.
        MarkedVia = "marked-via",
        /// `pub struct E(P0)`: a struct subject with its one field.
        Struct = "struct",
        /// `pub struct E { label: u64, #[dispatch] held: P0 }`: a struct
        /// subject with a marked field among several.
        StructMarked = "struct-marked",
        /// `pub struct E { held: P0 }`: a struct subject with one named
        /// field.
        StructNamed = "struct-named",
        /// `#[dispatch(via = *.payload)] A(Box<W0>)`: what the payload
        /// dereferences to, then a field of it, a route of two steps.
        DerefBoxField = "deref-box-field",
        /// `#[dispatch(via = *.payload)] A(Rc<W0>)`: the same behind a
        /// pointer that lends what it points at shared only.
        DerefRcField = "deref-rc-field",
        /// `#[dispatch(via = .1.payload)] A((u64, W0))`: a field of a
        /// field.
        ViaIndexField = "via-index-field",
        /// `#[dispatch(via = .get().payload)] A(W0)`: a field of what a
        /// method lends.
        ViaMethodField = "via-method-field",
        /// `#[dispatch(deref)] A(Box<dyn Probe>)`: the payload held as a
        /// trait object, the open set an enum holds beside its sized
        /// payloads (routes.md).
        DerefObject = "deref-dyn",
        /// `A { #[dispatch] held: P0 }`, a struct variant whose one field is
        /// named and marked, rebuilt by its braces (limits.md, routes.md).
        MarkedNamed = "marked-named",
        /// `A { label: u64, #[dispatch(deref)] held: Box<P0> }`, `deref` on
        /// a variant's marked field.
        MarkedDeref = "marked-deref",
        /// `A { label: u64, #[dispatch(Probe)] held: P0 }`, a trait named on
        /// a variant's marked field, which only a struct's field may name
        /// (limits.md).
        MarkedTrait = "marked-trait",
        /// `A { label: u64, #[dispatch(from = false)] held: P0 }`, a
        /// conversion option on a variant's marked field, which goes on the
        /// variant (limits.md).
        MarkedConversion = "marked-from",
        /// `A(u64, #[dispatch] P0)`, the marked field of a tuple variant
        /// that is its last, which a hand binds as `A(_, inner)` with no
        /// rest pattern left to skip anything.
        MarkedLast = "marked-last",
        /// `pub struct E(u64, #[dispatch] P0)`: a tuple struct subject whose
        /// marked field is its last.
        StructMarkedLast = "struct-marked-last",
        /// `pub struct E { #[dispatch] pub label: u64, #[dispatch(Probe)]
        /// pub held: P0 }`: a struct whose field names the trait under test,
        /// as its impl's home spells it, beside a bare field the trait would
        /// otherwise go to (a-fields-trait-may-be-a-partial-blocks.md).
        StructTrait = "struct-trait",
        /// The same struct whose home, the attribute or the block's header,
        /// spells the trait with one more path segment, `spelt::Probe`
        /// through a module of the cell that re-exports it: one name
        /// spelled two ways, the home's sentence
        /// (a-fields-trait-may-be-a-partial-blocks.md).
        StructTraitSpelt = "struct-trait-spelt",
        /// The same struct whose field names the trait by a path that
        /// begins with a keyword, `#[dispatch(self::Probe)]`, as its home
        /// spells it too: one spelling, so the trait goes to `held`.
        StructTraitSelf = "struct-trait-self",
        /// `#[dispatch(via = .lend(), via_mut = .lend_mut())] A(W0)`: a
        /// getter pair of a wrapper: "A method that borrows the target
        /// mutably follows `via_mut`. [..] Every other method follows `via`"
        /// (routes.md).
        ViaGetters = "via-getters",
        /// `A { label: u64, #[dispatch(via = .lend(), via_mut =
        /// .lend_mut())] held: W0 }`, the getter pair on a marked field.
        MarkedGetters = "marked-getters",
        /// `A(#[cfg_attr(all(), dispatch)] P0, u64)`, the mark of a tuple
        /// variant's field under `cfg_attr`: "You get an error on the mark,
        /// and the item is read as if the mark were plain" (limits.md).
        MarkedCfgAttr = "marked-cfg-attr",
        /// `pub struct E { pub label: u64, #[cfg_attr(all(), dispatch)] pub
        /// held: P0 }`, the same mark on a struct's field.
        StructMarkedCfgAttr = "struct-marked-cfg-attr",
    }
}

axis! {
    /// The second variant: a target kind over `P1`, no variant at all, or a
    /// variant no trait method can be forwarded to.
    pub enum Second {
        /// `B(P1)`.
        Plain = "plain",
        /// No second variant: a one-variant enum.
        Absent = "none",
        /// `#[dispatch(deref)] B(Box<P1>)`.
        DerefBox = "deref-box",
        /// `#[dispatch(deref)] B(Rc<P1>)`.
        DerefRc = "deref-rc",
        /// `#[dispatch(deref)] B(Arc<P1>)`.
        DerefArc = "deref-arc",
        /// `#[dispatch(deref)] B(&'a P1)`.
        DerefShared = "deref-ref",
        /// `#[dispatch(deref)] B(&'a mut P1)`.
        DerefExclusive = "deref-mut",
        /// `#[dispatch(via = .payload)] B(W1)`.
        ViaField = "via-field",
        /// `#[dispatch(via = .1)] B((u64, P1))`.
        ViaIndex = "via-index",
        /// `#[dispatch(via = .lend())] B(W1)`.
        ViaMethod = "via-method",
        /// `B(#[dispatch] P1, u64)`.
        MarkedTuple = "marked-tuple",
        /// `B { label: u64, #[dispatch] held: P1 }`.
        MarkedStruct = "marked-struct",
        /// `B { label: u64, #[dispatch(via = .payload)] held: W1 }`.
        MarkedVia = "marked-via",
        /// `B`, a unit variant.
        Unit = "unit",
        /// `B(P1, u64)`, two fields of which none is marked.
        Pair = "pair",
        /// `#[dispatch(via = *.payload)] B(Box<W1>)`.
        DerefBoxField = "deref-box-field",
        /// `#[dispatch(via = *.payload)] B(Rc<W1>)`.
        DerefRcField = "deref-rc-field",
        /// `#[dispatch(via = .1.payload)] B((u64, W1))`.
        ViaIndexField = "via-index-field",
        /// `#[dispatch(via = .get().payload)] B(W1)`.
        ViaMethodField = "via-method-field",
        /// `#[dispatch(deref)] B(Box<dyn Probe>)`, a `P1` held as a trait
        /// object.
        DerefObject = "deref-dyn",
        /// `#[dispatch(via = .lend(), via_mut = .lend_mut())] B(W1)`.
        ViaGetters = "via-getters",
    }
}

axis! {
    /// The receiver of the method under test.
    pub enum Receiver {
        /// `&self`.
        Shared = "ref",
        /// `&mut self`.
        Exclusive = "mut",
        /// `self`.
        Owned = "value",
        /// `self: Box<Self>`.
        Boxed = "box",
        /// `self: Pin<&Self>`.
        PinShared = "pin-ref",
        /// `self: Pin<&mut Self>`.
        PinExclusive = "pin-mut",
        /// `self: Rc<Self>`, which would need a refcount.
        Rc = "rc",
        /// `self: Arc<Self>`, which would need a refcount.
        Arc = "arc",
        /// `self: Pin<Box<Self>>`, which would need an allocation.
        PinBoxed = "pin-box",
    }
}

axis! {
    /// The return type of the method under test.
    pub enum Return {
        /// `u64`, a plain value.
        Tag = "u64",
        /// `()`.
        Unit = "unit",
        /// `&u64`, borrowed from the payload.
        Shared = "ref",
        /// `&mut u64`, borrowed mutably from the payload.
        Exclusive = "mut",
        /// `Self`.
        This = "self",
        /// `Option<Self>`.
        OptionThis = "option-self",
        /// `Result<Self, Fault>`.
        ResultThis = "result-self",
        /// `(Self, u64)`.
        TupleThis = "tuple-self",
        /// `Self::Out`, taken from the first variant.
        Assoc = "assoc",
        /// `Self::Out`, overridden on the enum with a type every payload's
        /// converts into.
        Out = "out",
        /// `Option<Self::Out>`, overridden.
        OptionOut = "option-out",
        /// `Result<u64, Self::Error>`, overridden.
        ResultError = "result-error",
        /// `Result<Self, Self::Error>`, overridden.
        ResultThisError = "result-self-error",
        /// `impl Future<Output = u64>`.
        Future = "future",
        /// `impl Future<Output = Self>`.
        FutureThis = "future-self",
        /// `Pin<Box<dyn Future<Output = u64> + Send + 'async_trait>>`, as
        /// `#[async_trait]` writes it, the references the method takes
        /// named and bound to outlive the future.
        BoxedFuture = "boxed-future",
        /// `Pin<Box<dyn Future<Output = Self::Out> + Send +
        /// 'async_trait>>`, overridden, so the output is converted.
        BoxedFutureOut = "boxed-future-out",
        /// `impl Iterator<Item = u64>`.
        Iterator = "iterator",
        /// `Vec<Self>`.
        VecThis = "vec-self",
        /// `Option<(Self, u64)>`.
        OptionTupleThis = "option-tuple-self",
        /// `support::fallible::Result<Self>`, a one-argument alias of
        /// `Result<Self, support::Fault>`, mapped like `Result<Self, E>`
        /// by the name its path ends in.
        FallibleThis = "fallible-self",
        /// `&'a u64`, borrowed for the subject's own lifetime, which an
        /// inherent place over a variant holding `&'a P0` answers.
        Lent = "lent",
        /// `Self::Item<'_>`, a generic associated type, which the enum
        /// declares in a partial block where it has one, `type Item<'v> =
        /// support::View<'v> where Self: 'v;`, every payload's own
        /// converting into it (partial-impl-blocks.md).
        Gat = "gat",
        /// `Self::Item<'_>` as `gat` declares it, which a partial block
        /// leaves undeclared.
        GatUndeclared = "gat-undeclared",
    }
}

axis! {
    /// A `Self` argument of the method under test, and where the result
    /// for two different variants, the mismatch, is spelled.
    pub enum Argument {
        /// No `Self` argument.
        Absent = "none",
        /// `&Self`, with `#[dispatch(mismatch = ..)]` on the trait method.
        SharedOnTrait = "ref-trait",
        /// `&Self`, with `mismatch(m = ..)` on the enum.
        SharedOnEnum = "ref-enum",
        /// `Self` by value, with `mismatch(m = ..)` on the enum.
        Owned = "value",
        /// `&mut Self`, with `#[dispatch(mismatch = ..)]` on the trait
        /// method.
        Exclusive = "mut",
        /// `&Self` with no mismatch anywhere.
        SharedBare = "ref-bare",
    }
}

axis! {
    /// What qualifies the method under test.
    pub enum Qualifier {
        /// A plain required method.
        Plain = "plain",
        /// `async fn`.
        Async = "async",
        /// `unsafe fn`.
        Unsafe = "unsafe",
        /// A generic method whose parameter an argument fixes.
        Generic = "generic",
        /// A generic method whose parameter no argument fixes, called with
        /// a turbofish.
        Turbofish = "turbofish",
        /// A provided method, with a default body, that `P0` overrides and
        /// `P1` does not.
        Provided = "provided",
        /// `fn m<T: Into<u64>>(.., x: T)` over a subject generic over `T`,
        /// or `fn m<P0: Into<u64>>(.., x: P0)` over the payload `P0`: a
        /// name the impl gives a meaning around the method, which the
        /// attribute renames as a hand does, `T2` or `P02`
        /// (what-is-generated.md).
        GenericNamed = "generic-named",
    }
}

axis! {
    /// How the method under test reaches the subject: the trait on the
    /// subject's attribute, a partial impl block, or an inherent block.
    pub enum Path {
        /// The trait named on the subject's `#[whichever::dispatch(..)]`.
        Attribute = "attribute",
        /// A partial impl block that says nothing about `m`.
        Block = "block",
        /// `#[dispatch(*)] fn m(..);` in the block.
        BlockStar = "block-star",
        /// `#[dispatch(A)] fn m(..) { .. }`: the first variant forwards, the
        /// body answers for the rest.
        BlockSome = "block-some",
        /// `#[dispatch(default)] fn m(..);`, left to the trait's default.
        BlockDefault = "block-default",
        /// The block writes `m` itself.
        BlockWritten = "block-written",
        /// An inherent block with `#[dispatch(*)] pub fn m(..);`, calling the
        /// payloads' own `m`.
        Inherent = "inherent",
        /// An inherent `#[dispatch(A)] pub fn m(..) { .. }`.
        InherentSome = "inherent-some",
        /// A partial impl block that says nothing about `m` and holds a
        /// macro invocation, which is not read (limits.md).
        BlockMacro = "block-macro",
        /// The same invocation under `#[cfg(all())]`, a `cfg` that holds:
        /// in the block, and reported, in this build (limits.md).
        BlockMacroOn = "block-macro-on",
        /// The same invocation under `#[cfg(any())]`, a `cfg` that does not
        /// hold: not in the block in this build, which is read without it
        /// and forwards as `block` does (limits.md).
        BlockMacroOff = "block-macro-off",
        /// `#[dispatch(*)] fn m(..) { match self { .. } }` in the block: the
        /// arms the enum handles itself, every other variant forwarded
        /// (partial-impl-blocks.md).
        BlockArms = "block-arms",
        /// An inherent `#[dispatch(*)] pub fn m(..) { match self { .. } }`
        /// (inherent-methods.md).
        InherentArms = "inherent-arms",
        /// A bodiless inherent `#[dispatch(held, label)] pub fn m(..);` on a
        /// struct, naming two of its fields, which a struct forwards a
        /// method to one of (limits.md).
        InherentFields = "inherent-fields",
        /// The same mark on the method in a partial block (limits.md).
        BlockFields = "block-fields",
        /// A partial block holding `m` without a body and without a mark
        /// under `#[cfg(all())]`, a `cfg` that holds: in the block, which
        /// cannot read it, and reported, in this build (limits.md).
        BlockBodilessOn = "block-bodiless-on",
        /// The same method under `#[cfg(any())]`, a `cfg` that does not
        /// hold: not in the block in this build, which is read without it
        /// and forwards as `block` does (limits.md).
        BlockBodilessOff = "block-bodiless-off",
    }
}

axis! {
    /// What an inherent method's arm calls or reads on its target.
    pub enum Callee {
        /// The payload's own method of the same name.
        Method = "method",
        /// The payload's method of another name, `call = answer`.
        Renamed = "renamed",
        /// The target itself, `value`.
        Value = "value",
        /// A field of the target, `field = state`.
        Field = "field",
        /// The payload's own method of the same name, taking `&self` where
        /// the method takes `self` by value, so that the arm borrows the
        /// target whatever holds it (the cameleon-genapi survey: a `Copy`
        /// enum over `&'a T` forwarding `fn base(self) -> &'a Base` to
        /// `T::base(&self)`).
        Borrowing = "borrowing",
    }
}

axis! {
    /// What an inherent method's `with` wraps each arm's result in.
    pub enum Lift {
        /// No `with`.
        Bare = "none",
        /// `with = Some`.
        Some = "some",
        /// `with = Ok`.
        Ok = "ok",
        /// `with = support::by_ref`, a function taking `&T`.
        ByRef = "by-ref",
        /// `with = support::by_value`, a function taking `T`.
        ByValue = "by-value",
        /// `with = Into::into`.
        Into = "into",
    }
}

axis! {
    /// The `#[cfg]` on the subject's variants.
    pub enum Cfg {
        /// No variant under `#[cfg]`.
        Absent = "none",
        /// The second variant under `#[cfg(all())]`, present.
        SecondOn = "second-on",
        /// The second variant under `#[cfg(any())]`, compiled out.
        SecondOff = "second-off",
        /// The first variant under `#[cfg(any())]`, compiled out.
        FirstOff = "first-off",
        /// Every variant under `#[cfg(all())]`, so the enum may be empty in
        /// some build and is matched by place.
        EveryOn = "every-on",
        /// Every variant under `#[cfg(any())]`, so the enum is empty in this
        /// build and only compiles if it is matched by place.
        EveryOff = "every-off",
    }
}

axis! {
    /// The attributes on the trait's method under test, and the bound it
    /// declares.
    pub enum Item {
        /// No attribute.
        Bare = "none",
        /// `#[cfg(all())]`: the method exists in this build.
        CfgOn = "cfg-on",
        /// `#[cfg(any())]`: the method exists in no build, and only the
        /// rest of the trait is forwarded.
        CfgOff = "cfg-off",
        /// `#[deprecated]`.
        Deprecated = "deprecated",
        /// `#[allow(non_snake_case)]`, beside an argument the lint would
        /// otherwise warn of.
        Allow = "allow",
        /// `#[inline(always)]`, which the forwarding takes instead of
        /// `#[inline]`; on a required method, "The carrier macro keeps it
        /// for them, and `#[dispatchable]` takes it off the trait's own
        /// method" (what-is-generated.md).
        Inline = "inline-always",
        /// `where Self: Sized`, which a trait object does not meet
        /// (limits.md).
        Sized = "sized",
    }
}

axis! {
    /// Where the trait and the payloads live, beside the subject.
    pub enum Home {
        /// In the cell's module, beside the subject.
        Local = "local",
        /// In a sibling module inside the cell, its signature naming a type
        /// by a `crate::` path.
        Sibling = "sibling",
        /// In another crate, a `traits-NNN` shard the subject's depends on,
        /// its signature naming a type by a `crate::` path of that crate.
        Crate = "crate",
    }
}

axis! {
    /// The generics of the trait or the subject.
    pub enum Generics {
        /// None.
        Bare = "none",
        /// The first variant's payload is a type parameter of the enum,
        /// `enum E<T> { A(T), .. }`.
        Payload = "payload",
        /// A generic trait dispatched at a concrete argument,
        /// `Probe<support::Json>`.
        Concrete = "concrete",
        /// A trait generic over a const dispatched at the enum's own
        /// parameter, `Probe<N>` on `enum E<const N: u64>`.
        Own = "own",
        /// A generic trait whose parameter has a default, left out,
        /// `#[dispatch(Probe)]` for `trait Probe<F = support::Json>`.
        Defaulted = "defaulted",
        /// A generic trait whose argument is a projection with generic
        /// arguments of its own, `F::Pair<F>`, dispatched at
        /// `Probe<support::Json>`.
        Paired = "paired",
        /// A trait with a lifetime parameter, `trait Probe<'t>`, which the
        /// enum names, `#[dispatch(Probe<'static>)]` (options.md: "Type,
        /// lifetime and const parameters are substituted").
        Lifetime = "lifetime",
        /// A trait generic over a format implemented for every `F` by a
        /// partial block's own header, `impl<F: support::Format<Value =
        /// u64>> Probe<F> for E {}`, with the bound the payloads' impls take
        /// (partial-impl-blocks.md, "The impl header"; golden
        /// `impl_header`).
        Every = "every",
        /// A generic enum, `enum E<T> { A(T), .. }`, whose partial block
        /// implements the trait for one instantiation, `impl Probe for
        /// E<P0> {}` (partial-impl-blocks.md, "The impl header").
        Instance = "instance",
    }
}

axis! {
    /// The conversions the subject's attribute writes, and which the law
    /// checks (options.md).
    pub enum Conversion {
        /// The default `From` per payload, unchecked.
        Unchecked = "none",
        /// The default `From` per payload, checked, and its absence where
        /// the docs say none is written, proven by a hand-written impl.
        From = "from",
        /// `from = false`, its absence proven.
        FromOff = "from-off",
        /// `try_from = true`.
        Try = "try",
        /// `try_from = ref`.
        TryRef = "try-ref",
        /// `try_from = ref_mut`.
        TryMut = "try-mut",
        /// `into = support::Sink`.
        Into = "into",
        /// `#[dispatch(from = false)]` on the first variant.
        VariantFromOff = "variant-from-off",
        /// `#[dispatch(try_from = true)]` on the first variant.
        VariantTry = "variant-try",
        /// `from = false` on the subject's attribute and on the second
        /// trait's: attributes that agree, one `from` (options.md).
        FromAgreed = "from-agreed",
        /// `from = true` on the subject's attribute and `from = false` on
        /// the second trait's, which contradicts it (options.md).
        FromContradicted = "from-contradicted",
        /// `into = support::Sink` on the subject's attribute and on the
        /// second trait's: one target given twice (options.md).
        IntoRepeated = "into-repeated",
        /// `into = support::Sink, into = support::Sink` on one attribute
        /// (options.md).
        IntoTwice = "into-twice",
    }
}

axis! {
    /// The trait's associated items beside those its return types name,
    /// and whether the payloads agree on them (what-is-generated.md, the
    /// first two rows of its table; options.md on `Name = value`).
    pub enum Agreement {
        /// No associated item of its own.
        Absent = "none",
        /// `type Kind;`, `u32` in `P0` and `u64` in `P1`, with no override.
        TypeDiffers = "type-differs",
        /// A const of every checked type, an integer, a float, `bool`,
        /// `char` and `&str`, on which the payloads agree.
        ConstAgrees = "const-agrees",
        /// The same consts, of which the integer differs in `P1`.
        ConstDiffers = "const-differs",
        /// `const PAIR: Option<u8>`, a type the check does not compare,
        /// different in each payload.
        ConstUnchecked = "const-unchecked",
        /// A `&str` const and a tuple const that differ in each payload,
        /// fixed on the enum with `NAME = "any"` and `RANGE = (support::LOW,
        /// support::HIGH)`.
        ConstOverride = "const-override",
        /// `type Kind;` as `type-differs` gives it, returned by a method
        /// every subject forwards, `fn kind(&self) -> Self::Kind`, whose
        /// arms meet the types the payloads give.
        TypeReturned = "type-returned",
        /// `fn label() -> u64 { 7 }`, a provided method without a receiver
        /// that every payload overrides: "On an enum, left to the trait's
        /// default body [..]. On a struct, `<Target as Trait>::method(args..)`"
        /// (what-is-generated.md).
        Function = "function",
    }
}

axis! {
    /// A second trait dispatched on the subject by an attribute of its own
    /// (options.md), with one method taking the control method's receiver,
    /// which the law checks as it checks the control method.
    pub enum Stack {
        /// No second trait.
        Absent = "none",
        /// `trait Other`, dispatched by a second `#[whichever::dispatch]`.
        Second = "second",
        /// `trait Other: Probe`, a supertrait of the trait under test.
        Supertrait = "supertrait",
        /// `trait Probe: Other`, a supertrait the trait under test requires,
        /// dispatched by one more attribute (options.md).
        Required = "required",
    }
}

axis! {
    /// The `#[deprecated]` on a variant of the subject
    /// (what-is-generated.md).
    pub enum Deprecation {
        /// No variant is deprecated.
        Absent = "none",
        /// The first variant is `#[deprecated]`.
        First = "first",
        /// The second variant is `#[deprecated]`.
        Second = "second",
    }
}

axis! {
    /// How the trait under test is declared (restatements.md).
    pub enum Declaration {
        /// `#[whichever::dispatchable] pub trait Probe`.
        Dispatchable = "dispatchable",
        /// The trait declared without the attribute, in another module of
        /// the cell or in the trait's crate, and restated beside the subject
        /// with `#[whichever::dispatchable(remote = path)]`, every method
        /// listed.
        Remote = "remote",
        /// The same restatement listing the control method alone, so the
        /// method under test, a provided one, is the real trait's default
        /// whatever its payload overrides.
        RemoteBase = "remote-base",
        /// `whichever::foreign::Display`, whose `fmt` takes `&self` and an
        /// argument; the payloads implement `core::fmt::Display`.
        Foreign = "foreign",
        /// `#[whichever::dispatchable] pub unsafe trait Probe`, whose every
        /// impl is an `unsafe impl` (what-is-generated.md).
        Unsafe = "unsafe",
        /// `whichever::foreign::Iterator`, of whose methods the law calls
        /// `next` and the provided `any`, which every payload overrides
        /// for `core::iter::Iterator`, so that the default the enum would
        /// otherwise answer with shows (foreign-traits-are-restated.md).
        ForeignIterator = "foreign-iterator",
    }
}

axis! {
    /// Which crate declares the enum the subject dispatches over
    /// (restatements.md, "Enums from other crates").
    pub enum Owner {
        /// The subject's crate: the subject is the enum.
        Local = "local",
        /// Another crate, the trait's crate shard, declares the enum generic
        /// over its payloads, and the subject restates it with
        /// `#[whichever::dispatch(remote = path)]`.
        Remote = "remote",
    }
}

axis! {
    /// The third variant, beside a second: a target kind over `P2`, or a
    /// variant no trait method can be forwarded to. A `match` of three
    /// arms, a mismatch over three variants, an agreement check across
    /// three payloads and a body beside two forwarded variants meet here.
    pub enum Third {
        /// No third variant.
        Absent = "none",
        /// `C(P2)`.
        Plain = "plain",
        /// `#[dispatch(deref)] C(Box<P2>)`.
        DerefBox = "deref-box",
        /// `#[dispatch(via = .payload)] C(W2)`.
        ViaField = "via-field",
        /// `C`, a unit variant.
        Unit = "unit",
    }
}

axis! {
    /// The arms a method under `*` writes in its body, on the paths that
    /// give it one (`block-arms`, `inherent-arms`): which variants they
    /// name, under which `#[cfg]`, and what else the body holds.
    pub enum Arms {
        /// An arm for the second variant, whatever it holds, and one for
        /// every later variant that holds no payload.
        Answered = "answered",
        /// The same, the second variant's arm under `#[cfg(all())]`.
        CfgOn = "cfg-on",
        /// The same, the second variant's arm under `#[cfg(any())]`, so the
        /// variant is forwarded in this build.
        CfgOff = "cfg-off",
        /// An arm for the second variant alone, which leaves a third that
        /// holds no payload without one.
        Second = "second",
        /// The arms of `answered` and a wildcard arm, which names no
        /// variant.
        Wildcard = "wildcard",
        /// A body that is no `match self`: the answer alone.
        Unmatched = "unmatched",
        /// The arms of `answered` behind a statement, `if false { return
        /// .. }`: a body with code before its `match self`, as the hand's
        /// method with a check before its `match` is written.
        Preceded = "preceded",
    }
}

impl Cfg {
    /// Returns the predicate the variant of a slot is under, if any.
    pub fn of(self, slot: usize) -> Option<&'static str> {
        match (self, slot) {
            (Cfg::Absent, _) => None,
            (Cfg::SecondOn, 1) | (Cfg::EveryOn, _) => Some("all()"),
            (Cfg::SecondOff, 1) | (Cfg::FirstOff, 0) | (Cfg::EveryOff, _) => Some("any()"),
            (Cfg::SecondOn | Cfg::SecondOff | Cfg::FirstOff, _) => None,
        }
    }
}
