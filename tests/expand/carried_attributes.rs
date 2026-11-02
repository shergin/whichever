//! What a trait item carries onto the impl: `unsafe`, a method's own
//! `#[inline]`, and the associated type taken from the first variant that
//! is always compiled in.
#[whichever::dispatchable]
pub unsafe trait Marker {
    type Tag;
    #[inline(always)]
    fn tag(&self) -> u8;
    #[inline(never)]
    fn slow(&self) -> u8;
    fn plain(&self) -> u8;
}

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

#[whichever::dispatch(Marker)]
pub enum Any {
    #[cfg(any())]
    Absent(Absent),
    Present(Present),
}

fn main() {}
