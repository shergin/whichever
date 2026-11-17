//! The open set: a `Box<dyn Trait>` in a `deref` variant beside sized
//! payloads. A method that requires `Self: Sized` cannot be called on the
//! trait object, so a partial block leaves it to the trait's default, and
//! the rest is forwarded. Where the box implements the trait itself, as a
//! `Box<dyn Iterator>` does, the variant holds it without `deref` and every
//! method is the box's.

/// A restatement of `Iterator` that lists the methods to forward; the rest
/// take the real trait's default. Three of the six require `Self: Sized`,
/// so the block leaves them to the default for the trait object.
mod restated {
    #[whichever::dispatchable(remote = core::iter::Iterator)]
    pub trait Iterator {
        type Item;
        fn next(&mut self) -> Option<Self::Item>;
        fn size_hint(&self) -> (usize, Option<usize>);
        fn count(self) -> usize
        where
            Self: Sized;
        fn last(self) -> Option<Self::Item>
        where
            Self: Sized;
        fn nth(&mut self, n: usize) -> Option<Self::Item>;
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B;
    }

    #[whichever::dispatch]
    pub enum AnyIter {
        Range(std::ops::Range<u32>),
        #[dispatch(deref)]
        Dyn(Box<dyn std::iter::Iterator<Item = u32>>),
    }

    #[whichever::dispatch]
    impl Iterator for AnyIter {
        #[dispatch(default)]
        fn count(self) -> usize;
        #[dispatch(default)]
        fn last(self) -> Option<u32>;
        #[dispatch(default)]
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            F: FnMut(B, u32) -> B;
    }

    #[test]
    fn sized_methods_take_the_default_and_the_rest_is_forwarded() {
        let mut dynamic = AnyIter::Dyn(Box::new(vec![1u32, 2, 3].into_iter()));
        assert_eq!(dynamic.size_hint(), (3, Some(3)));
        assert_eq!(dynamic.nth(1), Some(2));
        assert_eq!(dynamic.count(), 1);
        let range = AnyIter::Range(0..4);
        assert_eq!(range.last(), Some(3));
        let folded = AnyIter::Dyn(Box::new(1u32..4)).fold(0, |total, n| total * 10 + n);
        assert_eq!(folded, 123);
    }
}

/// `Box<dyn Iterator>` is itself an `Iterator`, so a variant that holds it
/// without `deref` takes every method of `foreign::Iterator`, and no
/// partial block is needed.
mod boxed {
    #[whichever::dispatch(whichever::foreign::Iterator)]
    pub enum AnyIter {
        Range(std::ops::Range<u32>),
        Dyn(Box<dyn Iterator<Item = u32>>),
    }

    fn dynamic() -> AnyIter {
        AnyIter::Dyn(Box::new(vec![1u32, 2, 3].into_iter()))
    }

    #[test]
    fn every_method_is_forwarded_to_the_box() {
        assert_eq!(dynamic().size_hint(), (3, Some(3)));
        assert_eq!(dynamic().count(), 3);
        assert_eq!(dynamic().last(), Some(3));
        assert_eq!(dynamic().fold(0, |total, n| total * 10 + n), 123);
        assert_eq!(dynamic().max(), Some(3));
        assert!(dynamic().any(|n| n == 2));
        assert_eq!(dynamic().collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(AnyIter::Range(0..4).sum::<u32>(), 6);
    }
}

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn doubled(&self) -> u32
    where
        Self: Sized,
    {
        self.area() * 2
    }
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

/// `Box<dyn Shape>` is no `Shape`, so `deref` is the way to it.
#[whichever::dispatch]
pub enum AnyShape {
    Plain(Square),
    #[dispatch(deref)]
    Boxed(Box<dyn Shape>),
}

#[whichever::dispatch]
impl Shape for AnyShape {
    #[dispatch(default)]
    fn doubled(&self) -> u32
    where
        Self: Sized;
}

#[test]
fn a_users_trait_object_takes_the_default_for_a_sized_method() {
    let boxed = AnyShape::Boxed(Box::new(Square(3)));
    assert_eq!((boxed.area(), boxed.doubled()), (9, 18));
    assert_eq!(AnyShape::Plain(Square(2)).doubled(), 8);
}

/// A trait object whose variant is compiled out of this build is no error
/// beside a method that requires `Self: Sized`: the sentence about it is
/// its variant's arm, under the variant's `cfg`, as any sentence about one
/// variant is, and the method is forwarded to the rest.
mod compiled_out {
    use super::Shape;

    pub struct Square(pub u32);

    impl Shape for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
    }

    #[whichever::dispatch(Shape)]
    pub enum Any {
        Plain(Square),
        #[cfg(not(test))]
        #[dispatch(deref)]
        Boxed(Box<dyn Shape>),
    }

    #[test]
    fn a_sized_method_is_forwarded_where_the_trait_object_is_compiled_out() {
        let plain = Any::Plain(Square(3));
        assert_eq!(plain.area(), 9);
        assert_eq!(plain.doubled(), 18);
    }
}
