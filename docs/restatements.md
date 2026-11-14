# Traits and enums from other crates

## Traits from `std` and other crates

`#[dispatch]` reads a trait's signatures from the carrier macro that
`#[dispatchable]` leaves next to the trait, so a trait from `std` or from a
crate you do not own cannot be dispatched as it is. Restate it once, with
`remote` naming the real trait, and dispatch the restatement:

```rust
#[whichever::dispatchable(remote = std::io::Write)]
trait Write {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    fn flush(&mut self) -> std::io::Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
}

#[whichever::dispatch(Write)] // implements std::io::Write
pub enum Stream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(UnixStream),
}
```

Nothing is emitted for the restatement but the macro, so the name is free
for `use std::io::Write;` in the same module. Exactly the methods restated
are forwarded, provided ones included; a changed signature fails to compile
against the real trait. Write the signature types in full, as
`whichever::foreign` does, or import them where the enum is: nothing but
the macro reads the restatement, so its own imports count as unused. Bounds
on a restated associated type may be left out. A restatement of an
`#[async_trait]` trait carries `#[async_trait]` above
`#[dispatchable(remote = ..)]`, as the trait itself does. The restatements an enum most often needs ship in
[`whichever::foreign`](https://docs.rs/whichever/latest/whichever/foreign/):
`Display`, `Debug`, `Error`, `PartialEq`, `Iterator`,
`DoubleEndedIterator`, `ExactSizeIterator`, `Read`, `Write`, `BufRead` and
`Seek`, each forwarding every provided method a `match` can forward. The
two iterator traits are dispatched beside `Iterator`, one attribute each,
so that `len`, `rev` and `next_back` are the payload's.
`#[dispatch(whichever::foreign::Display)]` on an error enum
replaces the usual hand-written `match`, and the variant that holds a
`String` keeps `From` away with `#[dispatch(from = false)]`:

```rust
#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
pub enum ClientError {
    Io(io::Error),
    Parse(ParseIntError),
    #[dispatch(from = false)]
    BadUrl(String),
}
```

`whichever::foreign::Error` forwards `source` the same way when every
payload is an error type. When one is not, as the `String` above, or when
the enum answers a method differently from every payload, that method goes
in a [partial impl block](partial-impl-blocks.md) over the
declaration: `#[dispatch] impl whichever::foreign::Error for ClientError
{ fn source(&self) -> .. { None } }`.

A restatement forwards only the methods it lists, so a provided method a
payload overrides belongs in it; left out, the enum answers with the
trait's default whatever its payload would. tokio's `AsyncWrite` is the
common case. Its streams answer `true` to `is_write_vectored` and take
every buffer in one `poll_write_vectored`, but a restatement of the three
required methods alone, as a minimal example writes it, has the enum
report `false`, and a caller such as hyper then copies every buffer into
one. Restate the two provided methods with the rest:

```rust
#[whichever::dispatchable(remote = tokio::io::AsyncWrite)]
trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>>;
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
    A: tokio::io::AsyncWrite + Unpin,
    B: tokio::io::AsyncWrite + Unpin,
{
}
```

A `self: Pin<&mut Self>` receiver needs every payload to be `Unpin`, and
the empty partial block gives the impl a header that says so, as
[the impl header](partial-impl-blocks.md#the-impl-header) describes.
Payloads that are type parameters would get overlapping `From<A>` and
`From<B>`, hence `from = false`. An enum over concrete streams,
`Tcp(TcpStream)` and `Tls(TlsStream<TcpStream>)`, needs neither: the
attribute `#[whichever::dispatch(AsyncWrite)]` on the enum is enough. The
same holds for any trait with provided methods a payload overrides, such as
the `futures` crate's `AsyncRead` and its `poll_read_vectored`.

## Enums from other crates

A trait of this crate may be forwarded over an enum another crate defines,
as a hand-written `impl LocalTrait for other::Either<L, R>` may: restate
the enum once, `remote` naming the real one, and dispatch the restatement.
Nothing is emitted for the restatement but its carrier and the impls,
written for the real path; a variant missing or misspelt fails against the
real enum. A restated enum gets no conversions and no inherent impl, which
the orphan rules and the language refuse; for a partial impl block, import
the real enum under the restatement's name and write the block on it:

```rust
use either::Either;

#[whichever::dispatch(remote = either::Either)]
#[whichever::dispatch(Describe)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

#[whichever::dispatch]
impl<L: Named, R: Named> Named for Either<L, R> {
    fn kind() -> &'static str {
        "either"
    }
}
```
