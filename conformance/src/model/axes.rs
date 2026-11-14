//! The axes of stage 1: trait dispatch through the enum's attribute.
//!
//! Each axis is declared here with its values and words; what a value
//! means is its descriptor, in the module of its kind: [`super::target`],
//! [`super::receiver`], [`super::shape`], [`super::argument`] and
//! [`super::qualifier`].

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
        /// `Pin<Box<dyn Future<Output = u64> + '_>>`.
        BoxedFuture = "boxed-future",
        /// `Pin<Box<dyn Future<Output = Self::Out> + '_>>`, overridden, so
        /// the output is converted.
        BoxedFutureOut = "boxed-future-out",
        /// `impl Iterator<Item = u64>`.
        Iterator = "iterator",
        /// `Vec<Self>`.
        VecThis = "vec-self",
        /// `Option<(Self, u64)>`.
        OptionTupleThis = "option-tuple-self",
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
    }
}
