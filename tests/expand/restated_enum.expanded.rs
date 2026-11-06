//! An enum another crate defines is restated with `remote`, and the impl
//! is written for the real path.
pub mod other {
    pub enum Pair<L, R> {
        Left(L),
        Right(R),
    }
}
pub trait Area {
    fn area(&self) -> u32;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Area_94ad65e16aae18b2_10_1 as Area;
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Pair_62721740f25e63cf_15_1 as Pair;
#[automatically_derived]
impl<L, R> Area for other::Pair<L, R>
where
    L: Area,
    R: Area,
{
    #[inline]
    fn area(&self) -> u32 {
        match self {
            other::Pair::Left(__inner) => <L as Area>::area(__inner),
            other::Pair::Right(__inner) => <R as Area>::area(__inner),
        }
    }
}
fn main() {}
