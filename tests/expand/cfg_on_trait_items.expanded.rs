//! A `#[cfg]` on a trait item is decided where the trait is: the generated
//! item exists exactly when the trait's does, and carries no `cfg`.
pub trait Counted {
    fn plain(&self) -> u32;
    fn present(&self) -> u32;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Counted_3722e28313d67adb_3_1 as Counted;
pub struct One;
impl Counted for One {
    fn plain(&self) -> u32 {
        1
    }
    fn present(&self) -> u32 {
        2
    }
}
pub enum Any {
    One(One),
}
#[automatically_derived]
impl ::core::convert::From<One> for Any {
    #[inline]
    fn from(__value: One) -> Self {
        Self::One(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Any_a6c163c752562339_23_1 as Any;
#[automatically_derived]
impl Counted for Any {
    #[inline]
    fn plain(&self) -> u32 {
        match self {
            Any::One(__inner) => <One as Counted>::plain(__inner),
        }
    }
    #[inline]
    fn present(&self) -> u32 {
        match self {
            Any::One(__inner) => <One as Counted>::present(__inner),
        }
    }
}
fn main() {}
