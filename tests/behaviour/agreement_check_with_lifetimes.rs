//! The agreement check lives in an impl on a wrapper of the enum, so the
//! enum's parameters are never passed to it and the bounds its payloads
//! imply hold there: a lifetime the first payload does not mention, and a
//! `T: 'a` implied by `&'a T`.

#[whichever::dispatchable]
pub trait Source {
    type Item;
    fn get(&self) -> Self::Item;
}

pub struct Owned(pub u8);

impl Source for Owned {
    type Item = u8;
    fn get(&self) -> u8 {
        self.0
    }
}

pub struct Borrowed<'a>(pub &'a u8);

impl Source for Borrowed<'_> {
    type Item = u8;
    fn get(&self) -> u8 {
        *self.0
    }
}

// The first payload has no lifetime; the second has the enum's.
#[whichever::dispatch(Source)]
pub enum OwnedFirst<'a> {
    Owned(Owned),
    Borrowed(Borrowed<'a>),
}

#[whichever::dispatch(Source)]
pub enum BorrowedFirst<'a> {
    Borrowed(Borrowed<'a>),
    Owned(Owned),
}

// `&'a T` in the payloads implies `T: 'a`, which the enum does not
// write and the check must not need.
#[whichever::dispatch(whichever::foreign::Iterator, from = false)]
pub enum Either<'a, T> {
    One(core::iter::Once<&'a T>),
    Many(core::slice::Iter<'a, T>),
}

#[whichever::dispatch(Source)]
pub enum Own<'a, T: Copy> {
    Slice(Slice<'a, T>),
    Opt(Opt<'a, T>),
}

pub struct Slice<'a, T>(pub &'a [T]);
pub struct Opt<'a, T>(pub Option<&'a T>);

impl<T: Copy> Source for Slice<'_, T> {
    type Item = Option<T>;
    fn get(&self) -> Option<T> {
        self.0.first().copied()
    }
}

impl<T: Copy> Source for Opt<'_, T> {
    type Item = Option<T>;
    fn get(&self) -> Option<T> {
        self.0.copied()
    }
}

#[test]
fn the_check_passes_whatever_the_first_payload_mentions() {
    let value = 7;
    assert_eq!(OwnedFirst::Borrowed(Borrowed(&value)).get(), 7);
    assert_eq!(BorrowedFirst::Owned(Owned(3)).get(), 3);
    let items = [1u32, 2, 3];
    let many: Either<'_, u32> = Either::Many(items.iter());
    assert_eq!(many.sum::<u32>(), 6);
    let one: Either<'_, u32> = Either::One(core::iter::once(&items[0]));
    assert_eq!(one.count(), 1);
    assert_eq!(Own::Slice(Slice(&items)).get(), Some(1));
    assert_eq!(Own::<u32>::Opt(Opt(None)).get(), None);
}
