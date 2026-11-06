//! What a trait item carries onto the impl: an `unsafe trait` gives an
//! `unsafe impl`, an associated item under `#[cfg]` stays under it, and the
//! associated items come from the first variant that is always compiled in.

/// A marker.
///
/// # Safety
///
/// Nothing to uphold; the trait is `unsafe` to show that the impl is.
#[whichever::dispatchable]
pub unsafe trait Marker {
    type Tag;
    #[cfg(any())]
    type Hidden;
    #[cfg(any())]
    const HIDDEN: u8;
    const TAG: u8;
    fn tag(&self) -> u8;
}

pub struct Absent;
pub struct Present;

unsafe impl Marker for Absent {
    type Tag = u16;
    const TAG: u8 = 1;
    fn tag(&self) -> u8 {
        1
    }
}

unsafe impl Marker for Present {
    type Tag = u8;
    const TAG: u8 = 2;
    fn tag(&self) -> u8 {
        2
    }
}

#[whichever::dispatch(Marker)]
pub enum Any {
    #[cfg(any())]
    Absent(Absent),
    Present(Present),
}

#[test]
fn the_first_compiled_in_variant_supplies_the_items() {
    let tag: <Any as Marker>::Tag = 7u8;
    assert_eq!(tag, 7);
    assert_eq!(<Any as Marker>::TAG, 2);
    assert_eq!(Any::Present(Present).tag(), 2);
}
