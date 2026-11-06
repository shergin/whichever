//! The open set: a `Box<dyn Trait>` in a `deref` variant beside sized
//! payloads. A method that requires `Self: Sized` cannot be called on the
//! trait object, so a partial block leaves it to the trait's default, and
//! the rest is forwarded.
use whichever::foreign::Iterator;

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
fn sized_methods_take_the_default_and_the_rest_is_forwarded() {
    let dynamic = AnyIter::Dyn(Box::new(vec![1u32, 2, 3].into_iter()));
    assert_eq!(dynamic.size_hint(), (3, Some(3)));
    assert_eq!(dynamic.count(), 3);
    let range = AnyIter::Range(0..4);
    assert_eq!(range.last(), Some(3));
    let boxed = AnyShape::Boxed(Box::new(Square(3)));
    assert_eq!((boxed.area(), boxed.doubled()), (9, 18));
    assert_eq!(AnyShape::Plain(Square(2)).doubled(), 8);
}
