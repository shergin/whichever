pub trait Proof {
    const KIND: &'static str;
    const ARITY: usize;
    fn check(&self) -> bool;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Proof_757e60a92f3e796a_1_1 as Proof;
pub struct Encoding;
pub struct Ordering;
impl Proof for Encoding {
    const KIND: &'static str = "encoding";
    const ARITY: usize = 1;
    fn check(&self) -> bool {
        true
    }
}
impl Proof for Ordering {
    const KIND: &'static str = "ordering";
    const ARITY: usize = 1;
    fn check(&self) -> bool {
        false
    }
}
pub enum AnyProof {
    Encoding(Encoding),
    Ordering(Ordering),
}
#[automatically_derived]
impl ::core::convert::From<Encoding> for AnyProof {
    #[inline]
    fn from(__value: Encoding) -> Self {
        Self::Encoding(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Ordering> for AnyProof {
    #[inline]
    fn from(__value: Ordering) -> Self {
        Self::Ordering(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnyProof_01d6d10efd4dbec6_27_1 as AnyProof;
#[automatically_derived]
impl Proof for AnyProof {
    const KIND: &'static str = "any";
    const ARITY: usize = <Encoding as Proof>::ARITY;
    #[inline]
    fn check(&self) -> bool {
        match self {
            AnyProof::Encoding(__inner) => <Encoding as Proof>::check(__inner),
            AnyProof::Ordering(__inner) => <Ordering as Proof>::check(__inner),
        }
    }
}
const _: () = {
    if !(<Encoding as Proof>::ARITY == <Ordering as Proof>::ARITY) {
        {
            ::core::panicking::panic_fmt(
                format_args!(
                    "`ARITY` of variant `Ordering` differs from the first variant\'s; set it on the enum with `ARITY = value`",
                ),
            );
        }
    }
};
fn main() {}
