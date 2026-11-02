#![allow(async_fn_in_trait)]
pub trait Format {
    type Value;
}
pub struct Json;
impl Format for Json {
    type Value = u32;
}
pub trait Encoder<F: Format> {
    type Error;
    async fn encode(&self, value: &F::Value) -> Result<String, Self::Error>;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Encoder_5d924be7c95fb5b7_13_1 as Encoder;
pub struct Compact;
pub struct Pretty;
impl Encoder<Json> for Compact {
    type Error = u8;
    async fn encode(&self, value: &u32) -> Result<String, u8> {
        Ok(value.to_string())
    }
}
impl Encoder<Json> for Pretty {
    type Error = u16;
    async fn encode(&self, value: &u32) -> Result<String, u16> {
        Ok(
            ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} ", value))
            }),
        )
    }
}
pub enum AnyEncoder {
    Compact(Compact),
    Pretty(Pretty),
}
#[automatically_derived]
impl ::core::convert::From<Compact> for AnyEncoder {
    #[inline]
    fn from(__value: Compact) -> Self {
        Self::Compact(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Pretty> for AnyEncoder {
    #[inline]
    fn from(__value: Pretty) -> Self {
        Self::Pretty(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnyEncoder_093a44e4abf4de6c_36_1 as AnyEncoder;
#[automatically_derived]
impl Encoder<Json> for AnyEncoder {
    type Error = u16;
    #[inline]
    async fn encode(
        &self,
        value: &<Json as Format>::Value,
    ) -> Result<String, Self::Error> {
        match self {
            AnyEncoder::Compact(__inner) => {
                <Compact as Encoder<Json>>::encode(__inner, value)
                    .await
                    .map_err(::core::convert::Into::into)
            }
            AnyEncoder::Pretty(__inner) => {
                <Pretty as Encoder<Json>>::encode(__inner, value)
                    .await
                    .map_err(::core::convert::Into::into)
            }
        }
    }
}
fn main() {}
