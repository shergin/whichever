use std::pin::Pin;
use std::task::{Context, Poll};
pub trait Source {
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32>;
    fn peek(self: Pin<&Self>) -> u32;
    fn finish(self: Box<Self>) -> u32;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Source_82ef634bdcec0cd0_4_1 as Source;
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
pub(crate) use __whichever_dispatch_AnySource_0e78ff65ae1eabf1_25_1 as AnySource;
#[automatically_derived]
impl Source for AnySource {
    #[inline]
    fn poll_next(self: Pin<&mut Self>, __argument0: &mut Context<'_>) -> Poll<u32> {
        match ::core::pin::Pin::get_mut(self) {
            AnySource::Boxed(__inner) => {
                <<Box<
                    Ready,
                > as ::core::ops::Deref>::Target as Source>::poll_next(
                    ::core::pin::Pin::new(&mut **__inner),
                    __argument0,
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
