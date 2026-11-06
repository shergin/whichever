//! `Result<Self, Self::Error>` and `Option<Self::Error>` with `Error`
//! overridden: the value is rewrapped and the error converted, each on its
//! own.

#[whichever::dispatchable]
pub trait Parse: Sized {
    type Error;
    fn reparse(&self) -> Result<Self, Self::Error>;
    fn error(&self) -> Option<Self::Error>;
    fn error_or(&self, fallback: u8) -> Result<u8, Self::Error>;
}

pub struct Text;
pub struct Binary;

#[derive(Debug, PartialEq)]
pub struct TextError;
#[derive(Debug, PartialEq)]
pub struct BinaryError;
#[derive(Debug, PartialEq)]
pub struct ParseError(&'static str);

impl From<TextError> for ParseError {
    fn from(_: TextError) -> Self {
        ParseError("text")
    }
}

impl From<BinaryError> for ParseError {
    fn from(_: BinaryError) -> Self {
        ParseError("binary")
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
    fn error_or(&self, fallback: u8) -> Result<u8, TextError> {
        Ok(fallback)
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
    fn error_or(&self, _fallback: u8) -> Result<u8, BinaryError> {
        Err(BinaryError)
    }
}

#[whichever::dispatch(Parse, Error = ParseError)]
pub enum AnyParser {
    Text(Text),
    Binary(Binary),
}

#[test]
fn the_value_is_rewrapped_and_the_error_converted() {
    let text = AnyParser::Text(Text);
    assert!(matches!(text.reparse(), Ok(AnyParser::Text(Text))));
    assert_eq!(text.error(), None);
    assert_eq!(text.error_or(7), Ok(7));
    let binary = AnyParser::Binary(Binary);
    assert!(matches!(binary.reparse(), Err(ParseError("binary"))));
    assert_eq!(binary.error(), Some(ParseError("binary")));
    assert_eq!(binary.error_or(7), Err(ParseError("binary")));
}
