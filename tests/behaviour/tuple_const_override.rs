//! A const override written as a tuple of paths parses as a type first; it
//! is read as a value once the trait says `RANGE` is a const.

#[whichever::dispatchable]
pub trait Ranged {
    const RANGE: (u8, u8);
}

pub const LOW: u8 = 1;
pub const HIGH: u8 = 9;

pub struct Narrow;
pub struct Wide;

impl Ranged for Narrow {
    const RANGE: (u8, u8) = (4, 5);
}

impl Ranged for Wide {
    const RANGE: (u8, u8) = (0, 255);
}

#[whichever::dispatch(Ranged, RANGE = (LOW, HIGH))]
pub enum AnyRanged {
    Narrow(Narrow),
    Wide(Wide),
}

#[test]
fn the_tuple_is_the_enums_const() {
    assert_eq!(<AnyRanged as Ranged>::RANGE, (1, 9));
}
