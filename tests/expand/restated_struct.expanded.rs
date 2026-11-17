//! A struct another crate defines is restated with `remote`, and the impls
//! are written for the real path; the patterns end in `..`, since the real
//! struct may have fields the restatement leaves out.
pub mod other {
    pub struct Square(pub u32);
    pub struct Solo(pub Square);
    pub struct Pair {
        pub left: Square,
        pub right: u8,
        pub note: &'static str,
    }
}
use other::Square;
pub trait Area {
    fn area(&self) -> u32;
    fn grown(&self) -> Self;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Area_aad34fba140864c6_16_1 as Area;
pub trait Tag {
    fn tag(&self) -> u8;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Tag_29bc18bf150d0c6a_22_1 as Tag;
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Solo_f214cc35a683ea14_27_1 as Solo;
#[automatically_derived]
impl Area for other::Solo {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            other::Solo(__inner, ..) => <Square as Area>::area(__inner),
        }
    }
    #[inline]
    fn grown(&self) -> Self {
        match self {
            other::Solo(__inner, ..) => other::Solo(<Square as Area>::grown(__inner)),
        }
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Pair_30e884e67458593e_30_1 as Pair;
#[automatically_derived]
impl Tag for other::Pair {
    #[inline]
    fn tag(&self) -> u8 {
        match self {
            other::Pair { right: __inner, .. } => <u8 as Tag>::tag(__inner),
        }
    }
}
fn main() {}
