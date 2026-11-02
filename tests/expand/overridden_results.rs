#[whichever::dispatchable]
pub trait Parse: Sized {
    type Error;
    fn reparse(&self) -> Result<Self, Self::Error>;
    fn error(&self) -> Option<Self::Error>;
}

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

#[whichever::dispatch(Parse, Error = ParseError)]
pub enum AnyParser {
    Text(Text),
    Binary(Binary),
}

fn main() {}
