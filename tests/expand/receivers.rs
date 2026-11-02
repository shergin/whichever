use std::pin::Pin;
use std::task::{Context, Poll};

#[whichever::dispatchable]
pub trait Source {
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32>;
    fn peek(self: Pin<&Self>) -> u32;
    fn finish(self: Box<Self>) -> u32;
}

pub struct Ready(pub u32);

impl Source for Ready {
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        Poll::Ready(self.0)
    }
    fn peek(self: Pin<&Self>) -> u32 {
        self.0
    }
    fn finish(self: Box<Self>) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Source)]
pub enum AnySource {
    #[dispatch(deref)]
    Boxed(Box<Ready>),
}

fn main() {}
