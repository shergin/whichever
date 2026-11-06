//! An empty partial impl block gives the impl a header the enum's attribute
//! cannot write: here, the trait's own parameter, for every `F`.
pub trait Format {
    type Value;
}
pub trait Encoder<F: Format> {
    fn encode(&self, value: &F::Value) -> usize;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Encoder_68367c70522778e6_7_1 as Encoder;
pub struct Json;
impl Format for Json {
    type Value = String;
}
pub struct A;
impl<F: Format> Encoder<F> for A {
    fn encode(&self, _: &F::Value) -> usize {
        1
    }
}
pub enum AnyEncoder {
    A(A),
}
#[automatically_derived]
impl ::core::convert::From<A> for AnyEncoder {
    #[inline]
    fn from(__value: A) -> Self {
        Self::A(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnyEncoder_13016b439300f773_24_1 as AnyEncoder;
impl<F: Format> Encoder<F> for AnyEncoder {
    #[inline]
    fn encode(&self, value: &<F as Format>::Value) -> usize {
        match self {
            AnyEncoder::A(__inner) => <A as Encoder<F>>::encode(__inner, value),
        }
    }
}
fn main() {}
