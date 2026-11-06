//! A tuple holding `Self` or an overridden associated type is taken apart,
//! each element converted, and built again.
pub trait Split: Sized {
    type Rest;
    fn split(&self) -> (Self, u32);
    fn halves(&self) -> (Self::Rest, Option<Self>);
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Split_3336cc2727a724db_3_1 as Split;
pub struct Even(pub u32);
impl Split for Even {
    type Rest = u8;
    fn split(&self) -> (Self, u32) {
        (Even(self.0 / 2), self.0 / 2)
    }
    fn halves(&self) -> (Self::Rest, Option<Self>) {
        (1, Some(Even(self.0)))
    }
}
pub enum Number {
    Even(Even),
}
#[automatically_derived]
impl ::core::convert::From<Even> for Number {
    #[inline]
    fn from(__value: Even) -> Self {
        Self::Even(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Number_27fa4764d48731c2_22_1 as Number;
#[automatically_derived]
impl Split for Number {
    type Rest = u32;
    #[inline]
    fn split(&self) -> (Self, u32) {
        match self {
            Number::Even(__inner) => {
                let (__element0, __element1) = <Even as Split>::split(__inner);
                (Number::Even(__element0), __element1)
            }
        }
    }
    #[inline]
    fn halves(&self) -> (Self::Rest, Option<Self>) {
        match self {
            Number::Even(__inner) => {
                let (__element0, __element1) = <Even as Split>::halves(__inner);
                (::core::convert::Into::into(__element0), __element1.map(Number::Even))
            }
        }
    }
}
fn main() {}
