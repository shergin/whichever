//! A marked unnamed field that is the last of its variant or struct is
//! matched without a rest pattern, which would have nothing to skip; a
//! marked field with others after it keeps its `..`.
pub trait Shape {
    fn area(&self) -> u64;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Shape_c0b023bf462e0816_4_1 as Shape;
pub struct Square(pub u64);
impl Shape for Square {
    fn area(&self) -> u64 {
        self.0 * self.0
    }
}
pub enum Tagged {
    Last(u64, Square),
    Middle(u8, Square, u64),
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Tagged_377b2c82853d9a62_17_1 as Tagged;
#[automatically_derived]
impl Shape for Tagged {
    #[inline]
    fn area(&self) -> u64 {
        match self {
            Tagged::Last(_, __inner) => <Square as Shape>::area(__inner),
            Tagged::Middle(_, __inner, ..) => <Square as Shape>::area(__inner),
        }
    }
}
pub struct Labelled(u64, Square);
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Labelled_37a50a3b300993fc_23_1 as Labelled;
#[automatically_derived]
impl Shape for Labelled {
    #[inline]
    fn area(&self) -> u64 {
        match self {
            Labelled(_, __inner) => <Square as Shape>::area(__inner),
        }
    }
}
fn main() {}
