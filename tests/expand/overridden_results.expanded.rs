pub trait Parse: Sized {
    type Error;
    fn reparse(&self) -> Result<Self, Self::Error>;
    fn error(&self) -> Option<Self::Error>;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Parse_b56a397b810b96f7_1_1 as Parse;
pub struct Text;
pub struct Binary;
pub struct TextError;
pub struct BinaryError;
pub struct ParseError;
impl From<TextError> for ParseError {
    fn from(_: TextError) -> Self {
        ParseError
    }
}
impl From<BinaryError> for ParseError {
    fn from(_: BinaryError) -> Self {
        ParseError
    }
}
impl Parse for Text {
    type Error = TextError;
    fn reparse(&self) -> Result<Self, TextError> {
        Ok(Text)
    }
    fn error(&self) -> Option<TextError> {
        None
    }
}
impl Parse for Binary {
    type Error = BinaryError;
    fn reparse(&self) -> Result<Self, BinaryError> {
        Err(BinaryError)
    }
    fn error(&self) -> Option<BinaryError> {
        Some(BinaryError)
    }
}
pub enum AnyParser {
    Text(Text),
    Binary(Binary),
}
#[automatically_derived]
impl ::core::convert::From<Text> for AnyParser {
    #[inline]
    fn from(__value: Text) -> Self {
        Self::Text(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Binary> for AnyParser {
    #[inline]
    fn from(__value: Binary) -> Self {
        Self::Binary(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnyParser_b31c50e0aba4c547_47_1 as AnyParser;
#[automatically_derived]
impl Parse for AnyParser {
    type Error = ParseError;
    #[inline]
    fn reparse(&self) -> Result<Self, Self::Error> {
        match self {
            AnyParser::Text(__inner) => {
                <Text as Parse>::reparse(__inner)
                    .map(AnyParser::Text)
                    .map_err(::core::convert::Into::into)
            }
            AnyParser::Binary(__inner) => {
                <Binary as Parse>::reparse(__inner)
                    .map(AnyParser::Binary)
                    .map_err(::core::convert::Into::into)
            }
        }
    }
    #[inline]
    fn error(&self) -> Option<Self::Error> {
        match self {
            AnyParser::Text(__inner) => {
                <Text as Parse>::error(__inner).map(::core::convert::Into::into)
            }
            AnyParser::Binary(__inner) => {
                <Binary as Parse>::error(__inner).map(::core::convert::Into::into)
            }
        }
    }
}
fn main() {}
