use std::pin::Pin;
use std::task::{Context, Poll};
pub trait Source {
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32>;
    fn peek(self: Pin<&Self>) -> u32;
    fn finish(self: Box<Self>) -> u32;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Source_f33c3f40e5468814_4_1 as Source;
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
pub enum AnySource {
    Boxed(Box<Ready>),
}
#[automatically_derived]
impl ::core::convert::From<Box<Ready>> for AnySource {
    #[inline]
    fn from(__value: Box<Ready>) -> Self {
        Self::Boxed(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnySource_3645bd574627a686_25_1 as AnySource;
#[automatically_derived]
impl Source for AnySource {
    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        match ::core::pin::Pin::get_mut(self) {
            AnySource::Boxed(__inner) => {
                <<Box<
                    Ready,
                > as ::core::ops::Deref>::Target as Source>::poll_next(
                    ::core::pin::Pin::new(&mut **__inner),
                    cx,
                )
            }
        }
    }
    #[inline]
    fn peek(self: Pin<&Self>) -> u32 {
        match ::core::pin::Pin::get_ref(self) {
            AnySource::Boxed(__inner) => {
                <<Box<
                    Ready,
                > as ::core::ops::Deref>::Target as Source>::peek(
                    ::core::pin::Pin::new(&**__inner),
                )
            }
        }
    }
    #[inline]
    fn finish(self: Box<Self>) -> u32 {
        match *self {
            AnySource::Boxed(__inner) => {
                <<Box<Ready> as ::core::ops::Deref>::Target as Source>::finish(__inner)
            }
        }
    }
}
fn main() {}
