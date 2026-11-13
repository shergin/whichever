//! `foreign::DoubleEndedIterator` and `foreign::ExactSizeIterator`.
//! The closed set is shaped like darling_core 0.24.1's private iterator:
//! a single item or a vector's, each trait dispatched beside `Iterator`.
//! The open set holds a `Box<dyn DoubleEndedIterator>` in a `deref`
//! variant, so the methods that require `Self: Sized` are left to the
//! trait's default in partial blocks.
use std::iter::{self, Once};
use std::ops::Range;
use std::vec;

#[whichever::dispatch(whichever::foreign::Iterator)]
#[whichever::dispatch(whichever::foreign::DoubleEndedIterator)]
#[whichever::dispatch(whichever::foreign::ExactSizeIterator)]
enum IntoIterEnum {
    Single(Once<u32>),
    Multiple(vec::IntoIter<u32>),
}

fn single() -> IntoIterEnum {
    IntoIterEnum::Single(iter::once(7))
}

fn multiple() -> IntoIterEnum {
    IntoIterEnum::Multiple(vec![1, 2, 3].into_iter())
}

fn digits(text: String, number: u32) -> String {
    format!("{text}{number}")
}

#[test]
fn size_hint_and_len_are_the_payloads_exact_ones() {
    assert_eq!(single().size_hint(), (1, Some(1)));
    assert_eq!(multiple().size_hint(), (3, Some(3)));
    assert_eq!(single().len(), 1);
    assert_eq!(multiple().len(), 3);
    let mut partly = multiple();
    partly.next_back();
    assert_eq!(partly.len(), 2);
}

#[test]
fn rev_next_back_and_nth_back_run_from_the_back() {
    assert_eq!(single().rev().collect::<Vec<_>>(), [7]);
    assert_eq!(multiple().rev().collect::<Vec<_>>(), [3, 2, 1]);
    let mut one = single();
    assert_eq!(one.next_back(), Some(7));
    assert_eq!(one.next_back(), None);
    let mut many = multiple();
    assert_eq!(many.next_back(), Some(3));
    assert_eq!(many.next(), Some(1));
    assert_eq!(single().nth_back(0), Some(7));
    assert_eq!(single().nth_back(1), None);
    assert_eq!(multiple().nth_back(1), Some(2));
}

#[test]
fn rfold_and_rfind_are_forwarded() {
    assert_eq!(single().rfold(String::new(), digits), "7");
    assert_eq!(multiple().rfold(String::new(), digits), "321");
    assert_eq!(single().rfind(|&number| number > 5), Some(7));
    assert_eq!(single().rfind(|&number| number < 5), None);
    let mut many = multiple();
    assert_eq!(many.rfind(|&number| number < 3), Some(2));
    assert_eq!(many.next_back(), Some(1));
}

#[whichever::dispatch]
enum AnyDoubleEnded {
    Range(Range<u32>),
    #[dispatch(deref)]
    Dyn(Box<dyn DoubleEndedIterator<Item = u32>>),
}

#[whichever::dispatch]
impl whichever::foreign::Iterator for AnyDoubleEnded {
    #[dispatch(default)]
    fn count(self) -> usize;
    #[dispatch(default)]
    fn last(self) -> Option<u32>;
    #[dispatch(default)]
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, u32) -> B;
}

#[whichever::dispatch]
impl whichever::foreign::DoubleEndedIterator for AnyDoubleEnded {
    #[dispatch(default)]
    fn rfold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, u32) -> B;
    #[dispatch(default)]
    fn rfind<P>(&mut self, predicate: P) -> Option<u32>
    where
        P: FnMut(&u32) -> bool;
}

fn range() -> AnyDoubleEnded {
    AnyDoubleEnded::Range(1..4)
}

fn boxed() -> AnyDoubleEnded {
    AnyDoubleEnded::Dyn(Box::new(vec![1u32, 2, 3].into_iter()))
}

#[test]
fn the_back_reaches_the_boxed_trait_object() {
    let mut dynamic = boxed();
    assert_eq!(dynamic.size_hint(), (3, Some(3)));
    assert_eq!(dynamic.next_back(), Some(3));
    assert_eq!(dynamic.next(), Some(1));
    assert_eq!(boxed().nth_back(2), Some(1));
    assert_eq!(boxed().nth_back(3), None);
    assert_eq!(boxed().rev().collect::<Vec<_>>(), [3, 2, 1]);
    assert_eq!(range().nth_back(1), Some(2));
    assert_eq!(range().rev().collect::<Vec<_>>(), [3, 2, 1]);
}

#[test]
fn rfold_and_rfind_take_the_default_on_both_variants() {
    assert_eq!(boxed().rfold(String::new(), digits), "321");
    assert_eq!(range().rfold(String::new(), digits), "321");
    let mut dynamic = boxed();
    assert_eq!(dynamic.rfind(|&number| number < 3), Some(2));
    assert_eq!(dynamic.next_back(), Some(1));
    assert_eq!(range().rfind(|&number| number > 5), None);
    assert_eq!(boxed().count(), 3);
}
