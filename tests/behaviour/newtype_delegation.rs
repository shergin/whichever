#[whichever::dispatchable]
trait FlowCtrl {
    fn can_send<M: AsRef<str>>(&self, msg: &M) -> bool;
    fn about_to_send(&mut self, bytes: u32);
    fn sent(&self) -> u32;
    fn label(&self) -> String {
        format!("{} sent", self.sent())
    }
    #[dispatch(mismatch = 0)]
    fn merged(&self, other: &Self) -> u32;
}

struct Window(u32);
struct XonXoff(u32);

impl FlowCtrl for Window {
    fn can_send<M: AsRef<str>>(&self, msg: &M) -> bool {
        msg.as_ref().len() as u32 <= self.0
    }
    fn about_to_send(&mut self, bytes: u32) {
        self.0 = self.0.saturating_sub(bytes);
    }
    fn sent(&self) -> u32 {
        100 - self.0
    }
    fn label(&self) -> String {
        String::from("window")
    }
    fn merged(&self, other: &Self) -> u32 {
        self.0 + other.0
    }
}

impl FlowCtrl for XonXoff {
    fn can_send<M: AsRef<str>>(&self, _msg: &M) -> bool {
        self.0 > 0
    }
    fn about_to_send(&mut self, _bytes: u32) {
        self.0 += 1;
    }
    fn sent(&self) -> u32 {
        self.0
    }
    fn merged(&self, other: &Self) -> u32 {
        self.0 * other.0
    }
}

#[whichever::dispatch(FlowCtrl)]
enum Inner {
    Window(Window),
    XonXoff(XonXoff),
}

/// The public wrapper over the private enum, with a field of its own.
#[whichever::dispatch(FlowCtrl)]
struct StreamFlowCtrl {
    #[dispatch]
    inner: Inner,
    hooks: u8,
}

/// A payload behind a `Box`.
#[whichever::dispatch(FlowCtrl)]
struct BoxedFlowCtrl {
    #[dispatch(deref)]
    inner: Box<Inner>,
}

/// A struct that overrides one method keeps the payload's overrides of
/// the others.
#[whichever::dispatch]
struct Newline(Window);

#[whichever::dispatch]
impl FlowCtrl for Newline {
    fn sent(&self) -> u32 {
        self.0.sent() + 1
    }
}

#[test]
fn a_struct_forwards_to_its_field_and_keeps_the_payloads_overrides() {
    let mut ctrl = StreamFlowCtrl {
        inner: Window(100).into(),
        hooks: 0,
    };
    assert!(ctrl.can_send(&"hello"));
    ctrl.about_to_send(40);
    assert_eq!((ctrl.sent(), ctrl.hooks), (40, 0));
    assert_eq!(ctrl.label(), "window");
    let other = StreamFlowCtrl {
        inner: Window(10).into(),
        hooks: 1,
    };
    assert_eq!(ctrl.merged(&other), 70);
    let mismatched = StreamFlowCtrl {
        inner: XonXoff(3).into(),
        hooks: 1,
    };
    assert_eq!(ctrl.merged(&mismatched), 0);

    let boxed = BoxedFlowCtrl {
        inner: Box::new(XonXoff(2).into()),
    };
    assert_eq!((boxed.sent(), boxed.label()), (2, String::from("2 sent")));

    let newline = Newline(Window(90));
    assert_eq!(
        (newline.sent(), newline.label()),
        (11, String::from("window"))
    );
}

#[whichever::dispatchable]
trait Grow {
    fn grown(&self) -> Self
    where
        Self: Sized;
    fn halved(&self) -> Option<Self>
    where
        Self: Sized;
}

#[derive(Debug, PartialEq)]
struct Size(u32);

impl Grow for Size {
    fn grown(&self) -> Self {
        Size(self.0 + 1)
    }
    fn halved(&self) -> Option<Self> {
        self.0.is_multiple_of(2).then_some(Size(self.0 / 2))
    }
}

/// A single-field struct needs no marker and is rebuilt around `Self`.
#[derive(Debug, PartialEq)]
#[whichever::dispatch(Grow)]
struct Wrapped(Size);

#[whichever::dispatch]
impl Wrapped {
    #[dispatch(*)]
    fn grown_again(&self) -> Self;
}

impl Size {
    fn grown_again(&self) -> Self {
        Size(self.0 + 2)
    }
}

#[test]
fn a_single_field_struct_is_rebuilt_around_self() {
    assert_eq!(Wrapped(Size(4)).grown(), Wrapped(Size(5)));
    assert_eq!(Wrapped(Size(4)).halved(), Some(Wrapped(Size(2))));
    assert_eq!(Wrapped(Size(3)).halved(), None);
    assert_eq!(Wrapped(Size(4)).grown_again(), Wrapped(Size(6)));
}

/// A struct whose one field is named rebuilds a bare `Self` with its
/// braces; a tuple struct rebuilds `Option<Self>` and `Result<Self, E>`
/// with its path as well.
mod rebuilt_around_one_field {
    #[whichever::dispatchable]
    pub trait Grow: Sized {
        fn grown(&self) -> Self;
        fn maybe_grown(&self) -> Option<Self> {
            None
        }
        fn tried_grown(&self) -> Result<Self, String> {
            Err(String::from("default"))
        }
        fn side(&self) -> u64;
    }

    pub struct Square(pub u64);

    impl Grow for Square {
        fn grown(&self) -> Self {
            Square(self.0 + 1)
        }
        fn maybe_grown(&self) -> Option<Self> {
            Some(Grow::grown(self))
        }
        fn tried_grown(&self) -> Result<Self, String> {
            Err(format!("{}", self.0))
        }
        fn side(&self) -> u64 {
            self.0
        }
    }

    #[whichever::dispatchable]
    pub trait Bare: Sized {
        fn grown(&self) -> Self;
        fn side(&self) -> u64;
    }

    impl Bare for Square {
        fn grown(&self) -> Self {
            Square(self.0 + 2)
        }
        fn side(&self) -> u64 {
            self.0
        }
    }

    #[whichever::dispatch(Bare)]
    pub struct Named {
        shape: Square,
    }

    #[whichever::dispatch(Grow)]
    pub struct Tuple(Square);

    #[test]
    fn a_named_field_rebuilds_a_bare_self() {
        let named = Named { shape: Square(1) };
        assert_eq!(Bare::side(&Bare::grown(&named)), 3);
    }

    #[test]
    fn a_tuple_struct_rebuilds_every_wrapper() {
        let tuple = Tuple(Square(1));
        assert_eq!(Grow::grown(&tuple).side(), 2);
        assert_eq!(tuple.maybe_grown().map(|grown| grown.side()), Some(2));
        assert_eq!(tuple.tried_grown().err(), Some(String::from("1")));
    }
}

/// A struct's target has a type the struct names, its one field or the
/// marked one, directly or through `deref`: a trait method without a
/// receiver is that type's, the provided ones included, and so is a
/// generic associated type.
mod functions_without_a_receiver {
    #[whichever::dispatchable]
    pub trait Kind {
        type Side<'a>
        where
            Self: 'a;
        type Paired<T>;
        fn kind() -> &'static str;
        fn label() -> &'static str {
            "kind"
        }
        fn side<'a>(&'a self) -> Self::Side<'a>;
        fn paired<T>(&self, with: T) -> Self::Paired<T>;
        async fn fetched(by: u32) -> u32;
        /// # Safety
        ///
        /// Nothing to uphold; the function is `unsafe` to show that the
        /// call is wrapped.
        unsafe fn raw() -> u32;
        fn converted<T: From<u8>>() -> T;
    }

    #[whichever::dispatchable]
    pub trait Unit {
        fn unit() -> Self;
    }

    #[whichever::dispatchable]
    pub trait Parse: Sized {
        fn parsed(text: &str) -> Option<Self>;
    }

    #[derive(Debug, PartialEq)]
    pub struct Square(pub u32);

    impl Kind for Square {
        type Side<'a> = &'a u32;
        type Paired<T> = (u32, T);
        fn kind() -> &'static str {
            "square"
        }
        fn label() -> &'static str {
            "a square"
        }
        fn side(&self) -> &u32 {
            &self.0
        }
        fn paired<T>(&self, with: T) -> (u32, T) {
            (self.0, with)
        }
        async fn fetched(by: u32) -> u32 {
            by + 1
        }
        unsafe fn raw() -> u32 {
            4
        }
        fn converted<T: From<u8>>() -> T {
            T::from(9)
        }
    }

    impl Unit for Square {
        fn unit() -> Self {
            Square(1)
        }
    }

    impl Parse for Square {
        fn parsed(text: &str) -> Option<Self> {
            text.parse().ok().map(Square)
        }
    }

    #[whichever::dispatch(Kind)]
    #[whichever::dispatch(Unit)]
    #[whichever::dispatch(Parse)]
    #[derive(Debug, PartialEq)]
    pub struct Tuple(pub Square);

    #[whichever::dispatch(Kind)]
    #[whichever::dispatch(Unit)]
    #[derive(Debug, PartialEq)]
    pub struct One {
        pub square: Square,
    }

    /// The marked field beside others: everything but `-> Self`, which
    /// would leave the other fields nothing.
    #[whichever::dispatch(Kind)]
    pub struct Labelled {
        #[dispatch]
        pub square: Square,
        pub label: &'static str,
    }

    #[whichever::dispatch(Kind)]
    pub struct Boxed(#[dispatch(deref)] pub Box<Square>);

    #[whichever::dispatch(Kind)]
    #[whichever::dispatch(Unit)]
    #[whichever::dispatch(Parse)]
    #[derive(Debug, PartialEq)]
    pub struct Generic<T>(pub T);

    fn functions<K: Kind>() -> (&'static str, &'static str, u32, u32, u64) {
        (
            K::kind(),
            K::label(),
            pollster::block_on(K::fetched(2)),
            unsafe { K::raw() },
            K::converted::<u64>(),
        )
    }

    #[test]
    fn a_function_is_the_field_types_and_so_is_its_override() {
        let expected = ("square", "a square", 3, 4, 9);
        assert_eq!(functions::<Tuple>(), expected);
        assert_eq!(functions::<One>(), expected);
        assert_eq!(functions::<Labelled>(), expected);
        assert_eq!(functions::<Boxed>(), expected);
        assert_eq!(functions::<Generic<Square>>(), expected);
    }

    #[test]
    fn self_is_built_around_the_one_field() {
        assert_eq!(Tuple::unit(), Tuple(Square(1)));
        assert_eq!(One::unit(), One { square: Square(1) });
        assert_eq!(Generic::<Square>::unit(), Generic(Square(1)));
        assert_eq!(Tuple::parsed("5"), Some(Tuple(Square(5))));
        assert_eq!(Tuple::parsed("five"), None);
        assert_eq!(Generic::<Square>::parsed("6"), Some(Generic(Square(6))));
    }

    #[test]
    fn a_generic_associated_type_is_the_field_types() {
        let tuple = Tuple(Square(2));
        let side: &u32 = tuple.side();
        assert_eq!(*side, 2);
        let boxed = Boxed(Box::new(Square(3)));
        assert_eq!(*boxed.side(), 3);
        let labelled = Labelled {
            square: Square(4),
            label: "four",
        };
        assert_eq!((*labelled.side(), labelled.label), (4, "four"));
        // `Generic<T>` declares `T`, so the generic associated type's own
        // `T` is renamed in the impl.
        let generic = Generic(Square(5));
        let paired: (u32, &str) = generic.paired("five");
        assert_eq!(paired, (5, "five"));
    }

    #[whichever::dispatchable]
    pub trait Tagged {
        fn label() -> &'static str {
            "default"
        }
        fn tag(&self) -> u32;
    }

    impl Tagged for Square {
        fn label() -> &'static str {
            "a square"
        }
        fn tag(&self) -> u32 {
            self.0
        }
    }

    pub struct Holder {
        pub square: Square,
    }

    /// Through a route the field's type cannot be named, so a provided
    /// function is left to the trait's default.
    #[whichever::dispatch(Tagged)]
    pub struct Routed {
        #[dispatch(via = .square)]
        pub holder: Holder,
    }

    #[test]
    fn a_provided_function_through_a_route_is_the_traits_default() {
        let routed = Routed {
            holder: Holder { square: Square(7) },
        };
        assert_eq!((Routed::label(), routed.tag()), ("default", 7));
    }
}
