//! `foreign::Iterator` forwards every provided method a `match` can
//! forward, so a payload's override is what runs through the enum or the
//! struct, and so is `core`'s: a slice iterator's `any` and `position`, a
//! range's `max`, `min` and `is_sorted`.
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;
use std::slice;
use std::vec;

/// Counts to three, and records each call of the two methods it
/// overrides; its `any` answers `true` whatever the predicate says.
pub struct Counting {
    count: u32,
    calls: Rc<Cell<u32>>,
}

impl Counting {
    fn new(calls: &Rc<Cell<u32>>) -> Self {
        Counting {
            count: 0,
            calls: Rc::clone(calls),
        }
    }
}

impl Iterator for Counting {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.count += 1;
        (self.count <= 3).then_some(self.count)
    }

    fn any<F: FnMut(u32) -> bool>(&mut self, _predicate: F) -> bool {
        self.calls.set(self.calls.get() + 1);
        true
    }

    fn for_each<F: FnMut(u32)>(self, mut f: F) {
        self.calls.set(self.calls.get() + 1);
        f(100);
    }
}

#[whichever::dispatch(whichever::foreign::Iterator)]
pub enum Source {
    Counting(Counting),
    Range(Range<u32>),
}

#[whichever::dispatch(whichever::foreign::Iterator)]
pub struct Wrapper {
    pub inner: Counting,
}

#[test]
fn a_payloads_any_and_for_each_run_through_the_enum() {
    let calls = Rc::new(Cell::new(0));
    let mut through = Source::Counting(Counting::new(&calls));
    assert!(through.any(|number| number > 10));
    let mut seen = Vec::new();
    Source::Counting(Counting::new(&calls)).for_each(|number| seen.push(number));
    assert_eq!(seen, [100]);
    assert_eq!(calls.get(), 2);
    assert!(!Source::Range(0..3).any(|number| number > 10));
}

#[test]
fn a_payloads_any_and_for_each_run_through_the_struct() {
    let calls = Rc::new(Cell::new(0));
    let mut through = Wrapper {
        inner: Counting::new(&calls),
    };
    assert!(through.any(|number| number > 10));
    let mut seen = Vec::new();
    Wrapper {
        inner: Counting::new(&calls),
    }
    .for_each(|number| seen.push(number));
    assert_eq!(seen, [100]);
    assert_eq!(calls.get(), 2);
}

#[whichever::dispatch(whichever::foreign::Iterator)]
pub enum Numbers<'a> {
    Range(Range<u32>),
    Owned(vec::IntoIter<u32>),
    Borrowed(std::iter::Copied<slice::Iter<'a, u32>>),
}

fn range() -> Numbers<'static> {
    Numbers::Range(1..5)
}

fn owned() -> Numbers<'static> {
    Numbers::Owned(vec![4, 1, 3, 2].into_iter())
}

#[test]
fn max_and_min_reach_the_range() {
    assert_eq!(range().max(), Some(4));
    assert_eq!(range().min(), Some(1));
    assert_eq!(owned().max(), Some(4));
    assert_eq!(owned().min(), Some(1));
    assert_eq!(owned().max_by_key(|&number| number % 4), Some(3));
    assert_eq!(owned().min_by_key(|&number| number % 4), Some(4));
    assert_eq!(owned().max_by(|a, b| b.cmp(a)), Some(1));
    assert_eq!(owned().min_by(|a, b| b.cmp(a)), Some(4));
}

#[test]
fn collect_sum_product_partition_and_reduce_are_forwarded() {
    assert_eq!(range().collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert_eq!(owned().collect::<Vec<_>>(), [4, 1, 3, 2]);
    assert_eq!(range().sum::<u32>(), 10);
    assert_eq!(owned().product::<u32>(), 24);
    let (even, odd): (Vec<u32>, Vec<u32>) = owned().partition(|number| number % 2 == 0);
    assert_eq!((even, odd), (vec![4, 2], vec![1, 3]));
    assert_eq!(range().reduce(|a, b| a * 10 + b), Some(1234));
}

#[test]
fn the_comparisons_are_forwarded() {
    use std::cmp::Ordering;
    assert_eq!(range().cmp(1..5), Ordering::Equal);
    assert_eq!(owned().cmp(range()), Ordering::Greater);
    assert_eq!(range().partial_cmp([1, 2, 4]), Some(Ordering::Less));
    assert!(range().eq([1, 2, 3, 4]));
    assert!(range().ne(owned()));
    assert!(range().lt(owned()));
    assert!(range().le(1..5));
    assert!(owned().gt(range()));
    assert!(owned().ge(owned()));
}

#[test]
fn is_sorted_is_forwarded() {
    assert!(range().is_sorted());
    assert!(!owned().is_sorted());
    assert!(owned().is_sorted_by(|a, b| a != b));
    assert!(range().is_sorted_by_key(|number| number / 2));
}

#[test]
fn the_searches_are_forwarded() {
    let numbers = [5u32, 7, 9];
    let borrowed = || Numbers::Borrowed(numbers.iter().copied());
    assert_eq!(borrowed().position(|number| number == 9), Some(2));
    assert_eq!(borrowed().find(|&number| number > 5), Some(7));
    assert_eq!(
        borrowed().find_map(|number| (number > 7).then_some(number * 2)),
        Some(18)
    );
    assert!(borrowed().all(|number| number % 2 == 1));
    assert!(!borrowed().any(|number| number == 6));
    let mut partly = range();
    assert_eq!(partly.position(|number| number == 2), Some(1));
    assert_eq!(partly.next(), Some(3));
}

/// A generic enum takes its header from a partial block, which says what
/// the two payloads share.
#[whichever::dispatch(from = false)]
pub enum Either<X, Y> {
    Left(X),
    Right(Y),
}

#[whichever::dispatch]
impl<X: Iterator, Y: Iterator<Item = X::Item>> whichever::foreign::Iterator for Either<X, Y> {}

#[test]
fn a_generic_enum_forwards_through_a_partial_block_header() {
    let calls = Rc::new(Cell::new(0));
    let mut left: Either<Counting, Range<u32>> = Either::Left(Counting::new(&calls));
    assert!(left.any(|number| number > 10));
    assert_eq!(calls.get(), 1);
    let right: Either<Counting, Range<u32>> = Either::Right(2..5);
    assert_eq!(right.max(), Some(4));
    let right: Either<Counting, Range<u32>> = Either::Right(2..5);
    assert_eq!(right.collect::<Vec<_>>(), [2, 3, 4]);
}

/// A generic struct forwards every method to its field.
#[whichever::dispatch(whichever::foreign::Iterator)]
pub struct Tagged<I> {
    pub inner: I,
}

#[test]
fn a_generic_struct_forwards_to_its_field() {
    let calls = Rc::new(Cell::new(0));
    let mut tagged = Tagged {
        inner: Counting::new(&calls),
    };
    assert!(tagged.any(|number| number > 10));
    assert_eq!(calls.get(), 1);
    assert_eq!(Tagged { inner: 1..4u32 }.sum::<u32>(), 6);
}
