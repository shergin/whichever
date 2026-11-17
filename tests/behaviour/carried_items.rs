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

/// A required method's `#[inline(..)]` is the forwarders': it leaves the
/// trait's own method, where rustc warns of it under `unused_attributes`,
/// and the module denies that lint to show the trait compiles clean.
#[deny(unused_attributes)]
mod a_required_methods_inline {
    #[whichever::dispatchable]
    pub trait Hot {
        #[inline(always)]
        fn heat(&self) -> u8;
        #[inline(never)]
        fn cool(&self) -> u8 {
            0
        }
    }

    pub struct Stove;

    impl Hot for Stove {
        fn heat(&self) -> u8 {
            9
        }
    }

    #[whichever::dispatch(Hot)]
    pub enum Kitchen {
        Stove(Stove),
    }

    #[test]
    fn the_trait_compiles_without_the_attribute_and_the_forwarder_runs() {
        assert_eq!(Kitchen::Stove(Stove).heat(), 9);
        assert_eq!(Kitchen::Stove(Stove).cool(), 0);
    }
}
