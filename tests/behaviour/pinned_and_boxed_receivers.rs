use std::pin::Pin;
use std::task::{Context, Poll, Waker};

#[whichever::dispatchable]
trait Source {
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32>;
    fn peek(self: Pin<&Self>) -> u32;
}

struct Ready(u32);
struct Pending;

impl Source for Ready {
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        Poll::Ready(self.0)
    }
    fn peek(self: Pin<&Self>) -> u32 {
        self.0
    }
}

impl Source for Pending {
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        Poll::Pending
    }
    fn peek(self: Pin<&Self>) -> u32 {
        0
    }
}

impl Ready {
    fn poll_once(self: Pin<&mut Self>) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Source)]
enum AnySource {
    Ready(Ready),
    Pending(Pending),
    #[dispatch(deref)]
    Boxed(Box<Ready>),
}

#[whichever::dispatch]
impl AnySource {
    #[dispatch(Ready, Boxed)]
    fn poll_once(self: Pin<&mut Self>) -> u32 {
        99
    }
}

#[whichever::dispatchable]
trait Finish {
    fn finish(self: Box<Self>) -> u32;
}

impl Finish for Ready {
    fn finish(self: Box<Self>) -> u32 {
        self.0 * 10
    }
}

#[whichever::dispatch(Finish)]
enum Finishable {
    #[dispatch(deref)]
    Boxed(Box<Ready>),
}

#[test]
fn pinned_receivers_unpin_the_enum_and_pin_the_payload() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut ready: AnySource = Ready(5).into();
    assert_eq!(Pin::new(&mut ready).poll_next(&mut cx), Poll::Ready(5));
    assert_eq!(Pin::new(&ready).peek(), 5);
    assert_eq!(Pin::new(&mut ready).poll_once(), 5);
    let mut pending: AnySource = Pending.into();
    assert_eq!(Pin::new(&mut pending).poll_next(&mut cx), Poll::Pending);
    assert_eq!(Pin::new(&mut pending).poll_once(), 99);
    let mut boxed = AnySource::Boxed(Box::new(Ready(7)));
    assert_eq!(Pin::new(&mut boxed).poll_once(), 7);
}

#[test]
fn a_boxed_receiver_hands_over_the_payloads_own_box() {
    let finishable = Box::new(Finishable::Boxed(Box::new(Ready(3))));
    assert_eq!(finishable.finish(), 30);
}

/// A small `Copy` target, read by value and by field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxedHandle(pub u64);

/// A boxed handle behind a field, which a route reaches with `.boxed*`.
pub struct BoxedHandleHolder {
    pub boxed: Box<BoxedHandle>,
}

// Under `self: Box<Self>` the enum's `match *self` binds each payload by
// value, so a `deref` variant's target is one dereference away from the
// binding, as under `self`. The other receivers of the same enum sit beside
// it, so every reading of the place is pinned together.
#[whichever::dispatch]
pub enum HandleSource {
    #[dispatch(deref)]
    Boxed(Box<BoxedHandle>),
    #[dispatch(via = .boxed*)]
    Held(BoxedHandleHolder),
    Plain(BoxedHandle),
}

// The boxed methods take the box only so that the arms written under it are
// tested; clippy's `boxed_local` is right that nothing else needs it.
#[whichever::dispatch]
#[allow(clippy::boxed_local)]
impl HandleSource {
    #[dispatch(*, value)]
    pub fn into_handle(self: Box<Self>) -> BoxedHandle;

    #[dispatch(*, field = 0)]
    pub fn into_raw(self: Box<Self>) -> u64;

    #[dispatch(*, value)]
    pub fn handle_owned(self) -> BoxedHandle;

    #[dispatch(*, field = 0)]
    pub fn raw_owned(self) -> u64;

    #[dispatch(*, value)]
    pub fn handle(&self) -> BoxedHandle;

    #[dispatch(*, value)]
    pub fn handle_ref(&self) -> &BoxedHandle;

    #[dispatch(*, field = 0)]
    pub fn raw(&self) -> u64;

    #[dispatch(*, value)]
    pub fn handle_mut(&mut self) -> &mut BoxedHandle;
}

fn handle_sources() -> [HandleSource; 3] {
    [
        HandleSource::Boxed(Box::new(BoxedHandle(1))),
        HandleSource::Held(BoxedHandleHolder {
            boxed: Box::new(BoxedHandle(2)),
        }),
        HandleSource::Plain(BoxedHandle(3)),
    ]
}

#[test]
fn a_boxed_receiver_reads_a_dereferenced_place_by_value() {
    let expected = [BoxedHandle(1), BoxedHandle(2), BoxedHandle(3)];
    let handles = handle_sources().map(|source| Box::new(source).into_handle());
    assert_eq!(handles, expected);
    let raws = handle_sources().map(|source| Box::new(source).into_raw());
    assert_eq!(raws, [1, 2, 3]);
    let handles = handle_sources().map(HandleSource::handle_owned);
    assert_eq!(handles, expected);
    let raws = handle_sources().map(HandleSource::raw_owned);
    assert_eq!(raws, [1, 2, 3]);
    let mut sources = handle_sources();
    let handles = sources.each_ref().map(HandleSource::handle);
    assert_eq!(handles, expected);
    let handles = sources.each_ref().map(|source| *source.handle_ref());
    assert_eq!(handles, expected);
    let raws = sources.each_ref().map(HandleSource::raw);
    assert_eq!(raws, [1, 2, 3]);
    for source in &mut sources {
        source.handle_mut().0 *= 10;
    }
    let handles = sources.each_ref().map(HandleSource::handle);
    assert_eq!(handles, [BoxedHandle(10), BoxedHandle(20), BoxedHandle(30)]);
}

// A place the return type borrows moves nothing out of the pointer, so under
// `self: Box<Self>` a `deref` variant holding a reference hands over the
// reference it holds, where a value taken out of it would be refused.
#[whichever::dispatch]
pub enum Lent<'a> {
    #[dispatch(deref)]
    Borrowed(&'a BoxedHandle),
    #[dispatch(deref)]
    Shared(&'a BoxedHandle),
    #[dispatch(via = .0*)]
    Paired((&'a BoxedHandle, u8)),
}

// The box is taken only so that the arm written under it is tested, as for
// `HandleSource` above.
#[whichever::dispatch]
#[allow(clippy::boxed_local)]
impl<'a> Lent<'a> {
    #[dispatch(*, value)]
    pub fn lent(self: Box<Self>) -> &'a BoxedHandle;
}

#[test]
fn a_boxed_receiver_hands_over_a_borrowed_place() {
    let first = BoxedHandle(1);
    let second = BoxedHandle(2);
    let third = BoxedHandle(3);
    let lent = Box::new(Lent::Borrowed(&first)).lent();
    assert!(std::ptr::eq(lent, &first));
    let lent = Box::new(Lent::Shared(&second)).lent();
    assert!(std::ptr::eq(lent, &second));
    let lent = Box::new(Lent::Paired((&third, 0))).lent();
    assert!(std::ptr::eq(lent, &third));
}
