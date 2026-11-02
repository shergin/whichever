pub trait Handler {
    type Value;
    fn make(seed: u32) -> Option<Self>
    where
        Self: Sized;
    fn value(&self) -> Self::Value;
    fn describe(&self) -> String {
        String::from("handler")
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Handler_48f915d41742c61d_1_1 as Handler;
pub struct Text;
pub struct Map;
impl Handler for Text {
    type Value = u32;
    fn make(_seed: u32) -> Option<Self> {
        Some(Text)
    }
    fn value(&self) -> u32 {
        1
    }
}
impl Handler for Map {
    type Value = u8;
    fn make(_seed: u32) -> Option<Self> {
        None
    }
    fn value(&self) -> u8 {
        2
    }
}
pub enum AnyHandler {
    Text(Text),
    Map(Map),
}
#[automatically_derived]
impl ::core::convert::From<Text> for AnyHandler {
    #[inline]
    fn from(__value: Text) -> Self {
        Self::Text(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Map> for AnyHandler {
    #[inline]
    fn from(__value: Map) -> Self {
        Self::Map(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnyHandler_b35853d5996e491b_36_1 as AnyHandler;
impl Handler for AnyHandler {
    #[inline]
    fn value(&self) -> Self::Value {
        match self {
            AnyHandler::Text(__inner) => {
                ::core::convert::Into::into(<Text as Handler>::value(__inner))
            }
            AnyHandler::Map(__inner) => {
                ::core::convert::Into::into(<Map as Handler>::value(__inner))
            }
        }
    }
    /// The enum's own value type; each payload's converts into it.
    type Value = u64;
    fn make(seed: u32) -> Option<Self> {
        Text::make(seed).map(AnyHandler::Text)
    }
    fn describe(&self) -> String {
        String::from("any handler")
    }
}
fn main() {}
