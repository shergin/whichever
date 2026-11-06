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
