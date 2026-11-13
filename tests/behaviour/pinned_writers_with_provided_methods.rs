//! A restated writer with pinned receivers forwards the provided methods it
//! lists, so a payload's vectored writes reach the enum; a restatement that
//! lists only the required methods leaves the enum on the trait's defaults.

use std::io::{IoSlice, Result};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// Stands for tokio, whose `AsyncWrite` has this exact shape.
pub mod io {
    use std::io::{IoSlice, Result};
    use std::pin::Pin;
    use std::task::{Context, Poll};

    pub trait AsyncWrite {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>>;

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<Result<usize>> {
            let buf = bufs
                .iter()
                .find(|buffer| !buffer.is_empty())
                .map_or(&[][..], |buffer| &**buffer);
            self.poll_write(cx, buf)
        }

        fn is_write_vectored(&self) -> bool {
            false
        }
    }
}

#[whichever::dispatchable(remote = io::AsyncWrite)]
pub trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize>>;
    fn is_write_vectored(&self) -> bool;
}

#[whichever::dispatch(from = false)]
pub enum CombinedStream<A, B> {
    A(A),
    B(B),
}

#[whichever::dispatch]
impl<A, B> AsyncWrite for CombinedStream<A, B>
where
    A: io::AsyncWrite + Unpin,
    B: io::AsyncWrite + Unpin,
{
}

/// A writer that reports vectored writes and takes every buffer in one call,
/// as tokio's `TcpStream` does.
#[derive(Default)]
pub struct Vectored {
    written: Vec<u8>,
    vectored_calls: usize,
    flushed: bool,
    shut_down: bool,
}

impl io::AsyncWrite for Vectored {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.written.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.flushed = true;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.shut_down = true;
        Poll::Ready(Ok(()))
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize>> {
        self.vectored_calls += 1;
        let mut length = 0;
        for buffer in bufs {
            self.written.extend_from_slice(buffer);
            length += buffer.len();
        }
        Poll::Ready(Ok(length))
    }

    fn is_write_vectored(&self) -> bool {
        true
    }
}

/// A writer that keeps both provided methods, so a vectored write takes only
/// the first non-empty buffer.
#[derive(Default)]
pub struct Plain {
    written: Vec<u8>,
}

impl io::AsyncWrite for Plain {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.written.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// The restatement a minimal example writes: the three required methods
/// only, so the enum keeps the trait's defaults for the provided two.
mod required_only {
    use super::io;
    use std::io::Result;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[whichever::dispatchable(remote = io::AsyncWrite)]
    pub trait AsyncWrite {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>>;
        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
    }

    #[whichever::dispatch(from = false)]
    pub enum CombinedStream<A, B> {
        A(A),
        B(B),
    }

    #[whichever::dispatch]
    impl<A, B> AsyncWrite for CombinedStream<A, B>
    where
        A: io::AsyncWrite + Unpin,
        B: io::AsyncWrite + Unpin,
    {
    }
}

fn buffers() -> [IoSlice<'static>; 3] {
    [
        IoSlice::new(b""),
        IoSlice::new(b"head"),
        IoSlice::new(b"tail"),
    ]
}

#[test]
fn is_write_vectored_reaches_the_enum_from_each_payload() {
    use io::AsyncWrite;
    let vectored: CombinedStream<Vectored, Plain> = CombinedStream::A(Vectored::default());
    assert!(vectored.is_write_vectored());
    let plain: CombinedStream<Vectored, Plain> = CombinedStream::B(Plain::default());
    assert!(!plain.is_write_vectored());
}

#[test]
fn poll_write_vectored_reaches_the_override_and_the_default() {
    use io::AsyncWrite;
    let mut cx = Context::from_waker(Waker::noop());

    let mut vectored: CombinedStream<Vectored, Plain> = CombinedStream::A(Vectored::default());
    let written = Pin::new(&mut vectored).poll_write_vectored(&mut cx, &buffers());
    assert!(matches!(written, Poll::Ready(Ok(8))));
    let CombinedStream::A(payload) = &vectored else {
        panic!("the variant changed");
    };
    assert_eq!(payload.vectored_calls, 1);
    assert_eq!(payload.written, b"headtail");

    let mut plain: CombinedStream<Vectored, Plain> = CombinedStream::B(Plain::default());
    let written = Pin::new(&mut plain).poll_write_vectored(&mut cx, &buffers());
    assert!(matches!(written, Poll::Ready(Ok(4))));
    let CombinedStream::B(payload) = &plain else {
        panic!("the variant changed");
    };
    assert_eq!(payload.written, b"head");
}

#[test]
fn the_required_methods_are_forwarded() {
    use io::AsyncWrite;
    let mut cx = Context::from_waker(Waker::noop());
    let mut stream: CombinedStream<Vectored, Plain> = CombinedStream::A(Vectored::default());
    let written = Pin::new(&mut stream).poll_write(&mut cx, b"body");
    assert!(matches!(written, Poll::Ready(Ok(4))));
    assert!(matches!(
        Pin::new(&mut stream).poll_flush(&mut cx),
        Poll::Ready(Ok(()))
    ));
    assert!(matches!(
        Pin::new(&mut stream).poll_shutdown(&mut cx),
        Poll::Ready(Ok(()))
    ));
    let CombinedStream::A(payload) = &stream else {
        panic!("the variant changed");
    };
    assert_eq!(payload.written, b"body");
    assert_eq!(payload.vectored_calls, 0);
    assert!(payload.flushed);
    assert!(payload.shut_down);
}

#[test]
fn a_restatement_of_the_required_methods_only_leaves_the_defaults() {
    use io::AsyncWrite;
    let mut cx = Context::from_waker(Waker::noop());
    let mut stream: required_only::CombinedStream<Vectored, Plain> =
        required_only::CombinedStream::A(Vectored::default());
    assert!(!stream.is_write_vectored());
    let written = Pin::new(&mut stream).poll_write_vectored(&mut cx, &buffers());
    assert!(matches!(written, Poll::Ready(Ok(4))));
    let required_only::CombinedStream::A(payload) = &stream else {
        panic!("the variant changed");
    };
    assert_eq!(payload.vectored_calls, 0);
    assert_eq!(payload.written, b"head");
}
