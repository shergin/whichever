//! The conversions are the enum's: they are written once however many
//! attributes the enum carries, and an option on any one attribute applies
//! to all of them. Two attributes may give `from` the same value; the
//! `try_from` forms add up, so `try_from = false` beside another form
//! cancels nothing; and the `into` targets of every attribute are collected.
//! A `from` that contradicts another and an `into` target named twice are
//! reported, in `tests/ui`.

use core::convert::TryFrom;

#[whichever::dispatchable]
trait Low {
    fn low(&self) -> u8;
}

#[whichever::dispatchable]
trait High {
    fn high(&self) -> u8;
}

struct One(u8);
struct Two(u8);

impl Low for One {
    fn low(&self) -> u8 {
        self.0
    }
}

impl High for One {
    fn high(&self) -> u8 {
        self.0 + 10
    }
}

impl Low for Two {
    fn low(&self) -> u8 {
        self.0
    }
}

impl High for Two {
    fn high(&self) -> u8 {
        self.0 + 20
    }
}

impl From<One> for u32 {
    fn from(one: One) -> Self {
        u32::from(one.0)
    }
}

impl From<Two> for u32 {
    fn from(two: Two) -> Self {
        u32::from(two.0) * 2
    }
}

impl From<One> for u64 {
    fn from(one: One) -> Self {
        u64::from(one.0) * 3
    }
}

impl From<Two> for u64 {
    fn from(two: Two) -> Self {
        u64::from(two.0) * 4
    }
}

#[whichever::dispatch(Low)]
#[whichever::dispatch(High)]
enum Plain {
    One(One),
    Two(Two),
}

// A hand-written `From` for each payload conflicts with a generated one, so
// these enums compile only when the attributes write none.
#[whichever::dispatch(Low, from = false)]
#[whichever::dispatch(High)]
enum OffOnTheFirst {
    One(One),
    Two(Two),
}

impl From<One> for OffOnTheFirst {
    fn from(one: One) -> Self {
        OffOnTheFirst::One(One(one.0 + 100))
    }
}

impl From<Two> for OffOnTheFirst {
    fn from(two: Two) -> Self {
        OffOnTheFirst::Two(Two(two.0 + 100))
    }
}

#[whichever::dispatch(Low)]
#[whichever::dispatch(High, from = false)]
enum OffOnTheSecond {
    One(One),
    Two(Two),
}

impl From<One> for OffOnTheSecond {
    fn from(one: One) -> Self {
        OffOnTheSecond::One(One(one.0 + 100))
    }
}

impl From<Two> for OffOnTheSecond {
    fn from(two: Two) -> Self {
        OffOnTheSecond::Two(Two(two.0 + 100))
    }
}

#[whichever::dispatch(Low, from = false)]
#[whichever::dispatch(High, from = false)]
enum OffOnBoth {
    One(One),
    Two(Two),
}

impl From<One> for OffOnBoth {
    fn from(one: One) -> Self {
        OffOnBoth::One(One(one.0 + 100))
    }
}

impl From<Two> for OffOnBoth {
    fn from(two: Two) -> Self {
        OffOnBoth::Two(Two(two.0 + 100))
    }
}

#[whichever::dispatch(Low, from = true)]
#[whichever::dispatch(High, from = true)]
enum OnOnBoth {
    One(One),
    Two(Two),
}

#[whichever::dispatch(Low, try_from = true)]
#[whichever::dispatch(High, try_from = false)]
enum TryBesideFalse {
    One(One),
    Two(Two),
}

#[whichever::dispatch(Low, try_from = true)]
#[whichever::dispatch(High)]
enum TryOnTheFirst {
    One(One),
    Two(Two),
}

#[whichever::dispatch(Low)]
#[whichever::dispatch(High, try_from = true)]
enum TryOnTheSecond {
    One(One),
    Two(Two),
}

#[whichever::dispatch(Low, try_from = ref)]
#[whichever::dispatch(High, try_from = true)]
enum WidestTry {
    One(One),
    Two(Two),
}

#[whichever::dispatch(Low, into = u32)]
#[whichever::dispatch(High, into = u64)]
enum IntoBoth {
    One(One),
    Two(Two),
}

#[test]
fn two_attributes_write_one_from_per_payload() {
    let one = Plain::from(One(1));
    assert_eq!((one.low(), one.high()), (1, 11));
    let two = Plain::from(Two(2));
    assert_eq!((two.low(), two.high()), (2, 22));
}

#[test]
fn from_false_on_either_attribute_leaves_the_from_impls_out() {
    assert_eq!(OffOnTheFirst::from(One(1)).high(), 111);
    assert_eq!(OffOnTheFirst::from(Two(2)).low(), 102);
    assert_eq!(OffOnTheSecond::from(One(1)).high(), 111);
    assert_eq!(OffOnTheSecond::from(Two(2)).low(), 102);
}

#[test]
fn try_from_on_either_attribute_adds_the_try_from_impls() {
    let Ok(one) = One::try_from(TryOnTheFirst::from(One(1))) else {
        panic!("`One` was not handed back");
    };
    assert_eq!(one.0, 1);
    let Err(back) = Two::try_from(TryOnTheFirst::from(One(1))) else {
        panic!("`One` was taken for `Two`");
    };
    assert_eq!(back.low(), 1);

    let Ok(two) = Two::try_from(TryOnTheSecond::from(Two(2))) else {
        panic!("`Two` was not handed back");
    };
    assert_eq!(two.0, 2);
    let Err(back) = One::try_from(TryOnTheSecond::from(Two(2))) else {
        panic!("`Two` was taken for `One`");
    };
    assert_eq!(back.high(), 22);
}

#[test]
fn the_widest_try_from_form_is_written() {
    let two = WidestTry::from(Two(2));
    let Ok(by_ref) = <&Two>::try_from(&two) else {
        panic!("`&Two` was not handed back");
    };
    assert_eq!(by_ref.0, 2);
    let Ok(owned) = Two::try_from(two) else {
        panic!("`Two` was not handed back");
    };
    assert_eq!(owned.0, 2);
}

#[test]
fn into_targets_of_both_attributes_are_written() {
    assert_eq!(u32::from(IntoBoth::from(Two(5))), 10);
    assert_eq!(u64::from(IntoBoth::from(Two(5))), 20);
    assert_eq!(u64::from(IntoBoth::from(One(5))), 15);
}

#[test]
fn two_attributes_that_agree_on_from_are_one_answer() {
    assert_eq!(OffOnBoth::from(One(1)).high(), 111);
    assert_eq!(OffOnBoth::from(Two(2)).low(), 102);
    let on = OnOnBoth::from(Two(2));
    assert_eq!((on.low(), on.high()), (2, 22));
}

#[test]
fn try_from_false_beside_another_form_cancels_nothing() {
    let Ok(one) = One::try_from(TryBesideFalse::from(One(1))) else {
        panic!("`One` was not handed back");
    };
    assert_eq!(one.0, 1);
}
