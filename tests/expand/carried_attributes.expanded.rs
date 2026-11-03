//! What a trait item carries onto the impl: `unsafe`, a method's own
//! `#[inline]` and `#[allow]`, and the associated type taken from the first
//! variant that is always compiled in.
pub unsafe trait Marker {
    #[allow(clippy::disallowed_types)]
    type Tag;
    #[inline(always)]
    fn tag(&self) -> u8;
    #[inline(never)]
    fn slow(&self) -> u8;
    #[allow(clippy::too_many_arguments)]
    fn plain(&self) -> u8;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Marker_bb4be0f98db5f7d3_4_1 as Marker;
pub struct Absent;
pub struct Present;
unsafe impl Marker for Absent {
    type Tag = u16;
    fn tag(&self) -> u8 {
        1
    }
    fn slow(&self) -> u8 {
        1
    }
    fn plain(&self) -> u8 {
        1
    }
}
unsafe impl Marker for Present {
    type Tag = u8;
    fn tag(&self) -> u8 {
        2
    }
    fn slow(&self) -> u8 {
        2
    }
    fn plain(&self) -> u8 {
        2
    }
}
pub enum Any {
    Present(Present),
}
#[automatically_derived]
impl ::core::convert::From<Present> for Any {
    #[inline]
    fn from(__value: Present) -> Self {
        Self::Present(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Any_cbaf65ef97868f71_45_1 as Any;
#[automatically_derived]
unsafe impl Marker for Any {
    #[allow(clippy::disallowed_types)]
    type Tag = <Present as Marker>::Tag;
    #[inline(always)]
    fn tag(&self) -> u8 {
        match self {
            Any::Present(__inner) => <Present as Marker>::tag(__inner),
        }
    }
    #[inline(never)]
    fn slow(&self) -> u8 {
        match self {
            Any::Present(__inner) => <Present as Marker>::slow(__inner),
        }
    }
    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn plain(&self) -> u8 {
        match self {
            Any::Present(__inner) => <Present as Marker>::plain(__inner),
        }
    }
}
const _: () = {
    #[allow(dead_code, non_snake_case)]
    fn __same_as_first_variant<
        __Variant: ?Sized + Marker<Tag = <Present as Marker>::Tag>,
    >() {}
    #[allow(dead_code, non_snake_case)]
    fn __whichever_associated_types_agree() {}
};
fn main() {}
