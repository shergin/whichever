# Traits, enums and structs from other crates

Often the trait you want on your enum is not yours. It is `Display`,
`Error`, `Iterator`, `Read` or `Write`, and it lives in `std`.
`whichever` ships ready-made restatements of these traits in
`whichever::foreign`, so you can forward them with one line:

```rust
use std::io;
use std::num::ParseIntError;

#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
pub enum ClientError {
    Io(io::Error),
    Parse(ParseIntError),
    #[dispatch(from = false)]
    BadUrl(String),
}
```

This replaces the usual hand-written `match` in `impl Display`. Notice
`#[dispatch(from = false)]` on the variant that holds a `String`: it keeps
that variant out of the `From` impls.

On this page:

- [What ships in `whichever::foreign`](#what-ships-in-whicheverforeign)
- [Why a trait from another crate needs this](#why-a-trait-from-another-crate-needs-this)
- [Restate any other trait](#restate-any-other-trait)
- [List the provided methods a payload overrides](#list-the-provided-methods-a-payload-overrides)
- [`Iterator` in detail](#iterator-in-detail)
- [`Error` in detail](#error-in-detail)
- [Enums from other crates](#enums-from-other-crates)

## What ships in `whichever::foreign`

| Restatement | Implements | Forwarded |
| --- | --- | --- |
| `Display` | `core::fmt::Display` | `fmt` |
| `Debug` | `core::fmt::Debug` | `fmt` |
| `Error` | `core::error::Error` | `source` |
| `PartialEq` | `core::cmp::PartialEq` | `eq` and `ne`; two different variants are never equal |
| `Iterator` | `core::iter::Iterator` | `type Item`, `next`, `size_hint`, `count`, `last`, `nth`, `for_each`, `collect`, `partition`, `fold`, `reduce`, `all`, `any`, `find`, `find_map`, `position`, `max`, `min`, `max_by_key`, `max_by`, `min_by_key`, `min_by`, `sum`, `product`, `cmp`, `partial_cmp`, `eq`, `ne`, `lt`, `le`, `gt`, `ge`, `is_sorted`, `is_sorted_by`, `is_sorted_by_key` |
| `DoubleEndedIterator` | `core::iter::DoubleEndedIterator` | `next_back`, `nth_back`, `rfold`, `rfind` |
| `ExactSizeIterator` | `core::iter::ExactSizeIterator` | `len` |
| `Read` | `std::io::Read` | `read`, `read_vectored`, `read_to_end`, `read_to_string`, `read_exact` |
| `Write` | `std::io::Write` | `write`, `write_vectored`, `flush`, `write_all`, `write_fmt` |
| `BufRead` | `std::io::BufRead` | `fill_buf`, `consume`, `read_until`, `skip_until`, `read_line` |
| `Seek` | `std::io::Seek` | `seek`, `rewind`, `stream_position`, `seek_relative` |

Each restatement forwards every provided method (a method with a default
body) that a `match` can forward and that is stable on the crate's minimum Rust
version. The one exception is `Error`, which leaves out its two deprecated
methods (see [`Error` in detail](#error-in-detail)). The full API is on
[docs.rs](https://docs.rs/whichever/latest/whichever/foreign/).

## Why a trait from another crate needs this

`#[dispatch]` does not look at a trait directly. It reads the trait's
signatures from the [carrier macro](glossary.md#carrier-macro): a hidden
macro that `#[dispatchable]` leaves next to the trait. A trait in `std`, or
in any crate you do not own, never had `#[dispatchable]` on it, so it has
no carrier macro, and `#[dispatch]` cannot forward it as it is.

The fix is a [restatement](glossary.md#restatement): a copy of the trait's
signatures, marked `remote`, that gives `#[dispatch]` something to read.
Everything in `whichever::foreign` is a restatement written for you.

## Restate any other trait

For a trait that `whichever::foreign` does not cover, restate it once with
`remote` set to the path of the real trait, and dispatch the restatement:

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

The impl you get is of the real `std::io::Write`. The rules:

- **Nothing is emitted for the restatement but the carrier macro.** There
  is no new trait called `Write`, so the name stays free for
  `use std::io::Write;` in the same module.
- **Exactly the restated methods are forwarded**, provided ones included.
  A method you leave out keeps the real trait's default (see the next
  section for why that matters).
- **A changed signature fails to compile against the real trait**, because
  the impl is written for the real trait.
- **Write the signature types in full**, as `whichever::foreign` does, or
  import them where the enum is. Nothing but the macro reads the
  restatement, so the restatement's own imports count as unused.
- **Bounds on a restated associated type may be left out.**
- **An `#[async_trait]` trait**: put `#[async_trait]` above
  `#[dispatchable(remote = ..)]` on the restatement, in the same order as
  on the trait itself.

## List the provided methods a payload overrides

A restatement forwards only the methods it lists. A provided method you
leave out gets the trait's default on the enum, whatever the payload does.
So if a payload overrides a provided method, that method belongs in the
restatement.

tokio's `AsyncWrite` is the common case. Its streams return `true` from
`is_write_vectored` and take every buffer in one `poll_write_vectored`
call. A restatement of only the three required methods, as a minimal
example writes it, makes the enum report `false`. A caller such as hyper
then copies every buffer into one before writing it. Restate the two
provided methods with the rest:

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

Two things in this example are there because the payloads are type
parameters:

- **The empty partial impl block.** A `self: Pin<&mut Self>` receiver
  needs every payload to be `Unpin`. The empty
  [partial impl block](partial-impl-blocks.md) gives the impl a header
  that says so, as [the impl header](partial-impl-blocks.md#the-impl-header)
  describes.
- **`from = false`.** Payloads that are type parameters would get
  overlapping `From<A>` and `From<B>` impls, so the enum turns them off.

An enum over concrete streams, such as `Tcp(TcpStream)` and
`Tls(TlsStream<TcpStream>)`, needs neither. The attribute
`#[whichever::dispatch(AsyncWrite)]` on the enum is enough.

The same advice applies to any trait with provided methods that a payload
overrides, such as the `futures` crate's `AsyncRead` and its
`poll_read_vectored`.

## `Iterator` in detail

### What is forwarded, and why it matters

`whichever::foreign::Iterator` forwards `next` and `size_hint`, and also
`count`, `last`, `nth`, `for_each`, `collect`, `partition`, `fold`,
`reduce`, `all`, `any`, `find`, `find_map`, `position`, `max`, `min`, the
`_by` and `_by_key` forms of both, `sum`, `product`, the comparisons
`cmp`, `partial_cmp`, `eq`, `ne`, `lt`, `le`, `gt` and `ge`, and
`is_sorted` with its two forms.

Because they are forwarded, a payload's own override is what runs:

- `core`'s slice iterators implement `any`, `all`, `find`, `find_map` and
  `position` themselves.
- A range implements `max`, `min` and `is_sorted` itself.
- A `vec::IntoIter` collected into a `Vec` reuses its allocation.

There is a speed reason too. The defaults of `all`, `any`, `find` and
`position` run through `try_fold`, which cannot be restated. On the enum,
those defaults would `match` once per item. Forwarded, they are one
`match`.

### What is left out, and why

- **The adapters that wrap `Self`**, such as `map`, `filter`, `rev` and
  the rest, **and `by_ref`.** They keep their defaults, which call the
  forwarded methods.
- **`unzip`**, which a `match` cannot forward. Its bound
  `Self: Iterator<Item = (A, B)>` does not reach the payload.
- **`rposition`.** Its bound `Self: ExactSizeIterator +
  DoubleEndedIterator` is true only for an enum that is both.
- **`try_fold`, `try_for_each`, `advance_by`** and the others that are
  unstable on the crate's minimum Rust version.

### `Self: Sized` and boxed iterators

Every method the `Iterator` restatement forwards, except `next`,
`size_hint` and `nth`, needs `Self: Sized`, as `core` declares it.

An enum that holds a `Box<dyn Iterator>` holds it as a plain payload,
without `deref`. The box is itself an `Iterator`, so every method is
forwarded to it, and you need no partial impl block. The same goes for a
`Box<dyn DoubleEndedIterator>` under `DoubleEndedIterator` and a
`Box<dyn ExactSizeIterator>` under `ExactSizeIterator`.

### `DoubleEndedIterator` and `ExactSizeIterator`

Dispatch these beside `Iterator`, one attribute each. Then `len`, `rev`
and `next_back` run the payload's own code.

### Payloads with different items

An [override](glossary.md#override) such as `Item = Wide` converts what a
method returns. It does not convert an argument: the argument goes to the
payload as it is. So the methods that take a closure over `Self::Item`
compile only when every payload's `Item` is `Wide`.

When the payloads' items differ, restate the trait yourself with only the
methods to forward. The rest take the real trait's default, which calls
them:

```rust
#[whichever::dispatchable(remote = core::iter::Iterator)]
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
    fn size_hint(&self) -> (usize, Option<usize>);
}

#[whichever::dispatch(Iterator, Item = Wide)]
pub enum Either {
    Narrow(Range<u32>),
    Broad(Range<u64>),
}
```

## `Error` in detail

`whichever::foreign::Error` forwards `source` to the payload, the same way
`Display` forwards `fmt`, when every payload is an error type.

It leaves out `description` and `cause`, which are deprecated. You would
not forward them by hand. And if they were in the restatement, a partial
impl block that writes `source` itself would have to write them too.

When one payload is not an error type, like the `String` in `ClientError`
above, or when the enum handles a method differently from every payload,
write that method in a [partial impl block](partial-impl-blocks.md) over
the restatement:

```rust
#[whichever::dispatch]
impl whichever::foreign::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientError::Io(error) => Some(error),
            _ => None,
        }
    }
}
```

The block refers to the restatement, and the impl you get is of the real
`core::error::Error`.

## Enums from other crates

The same trick works the other way round. You can forward a trait of your
crate over an enum another crate defines, as a hand-written
`impl LocalTrait for other::Either<L, R>` could. Restate the enum once,
with `remote` set to the real one, and dispatch the restatement:

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

What you get:

- **Nothing is emitted for the restatement but its carrier macro and the
  impls**, and the impls are written for the real path, `either::Either`.
- **A missing or misspelt variant fails to compile** against the real
  enum.
- **No conversions and no inherent impl.** The orphan rules and the
  language refuse both for a type from another crate.
- **For a partial impl block, import the real enum under the
  restatement's name** and write the block on it, as the `use
  either::Either;` and the `Named` block above do.

## Structs from other crates

A struct another crate defines is restated the same way: put `remote` on
the struct's attribute, naming the real struct, and list the fields the
impls forward to. The fields are forwarded as a
[dispatch struct](structs.md)'s are: the one field, the field marked
`#[dispatch]`, or a field that lists the trait.

```rust
use other::Pair;

// `other::Pair` also has a `note: &'static str` that the impls do not need.
#[whichever::dispatch(remote = other::Pair)]
#[whichever::dispatch(Area)]
#[whichever::dispatch(Tag)]
pub struct Pair {
    #[dispatch]
    left: Square,
    #[dispatch(Tag)]
    right: u8,
}
```

What you get:

- **The impls are written for the real struct**, as for an enum, and
  nothing else is emitted for the restatement but its carrier macro.
- **The restatement lists only the fields it forwards to.** The real
  struct may have more: the pattern ends in `..`, as yours would over a
  struct you do not own, `other::Pair { left: inner, .. }`. That is the
  difference from an enum, whose `match` has to name every variant.
- **A misspelt field, or a field of another type, fails to compile**
  against the real struct. A field you leave out is not caught.
- **`-> Self` is built around the real struct**, so it fails to compile
  when the real struct has fields that the restatement leaves out.
- **No inherent impl and no conversions**, as for an enum. The language
  refuses an inherent impl of another crate's type, and no struct gets
  conversions.
- **For a partial impl block, import the real struct under the
  restatement's name**, as for an enum.

### The fine print

`remote` restates the type, so it goes on the enum's or the struct's own
attribute. On a partial impl block's attribute it is an error: a block's
attribute takes the trait's options only.
