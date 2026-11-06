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
