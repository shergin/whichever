//! A payload that is a type parameter of the enum gets the bound a hand
//! writes on the impl, behind a pointer too.
pub trait Weigh {
    fn weight(&self) -> u32;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Weigh_f27adf9450d1aacd_3_1 as Weigh;
pub enum Lazy<Raw, Parsed> {
    Raw(Raw),
    Parsed(Box<Parsed>),
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Lazy_d6a93dcd26f41e0e_8_1 as Lazy;
#[automatically_derived]
impl<Raw, Parsed> Weigh for Lazy<Raw, Parsed>
where
    Raw: Weigh,
    Parsed: Weigh,
{
    #[inline]
    fn weight(&self) -> u32 {
        match self {
            Lazy::Raw(__inner) => <Raw as Weigh>::weight(__inner),
            Lazy::Parsed(__inner) => {
                <<Box<
                    Parsed,
                > as ::core::ops::Deref>::Target as Weigh>::weight(&**__inner)
            }
        }
    }
}
pub struct Wrapper<T>(T);
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Wrapper_dd7a094c91200ce5_15_1 as Wrapper;
#[automatically_derived]
impl<T> Weigh for Wrapper<T>
where
    T: Weigh,
{
    #[inline]
    fn weight(&self) -> u32 {
        match self {
            Wrapper(__inner, ..) => <T as Weigh>::weight(__inner),
        }
    }
}
fn main() {}
