//! A partial impl block over a `foreign` declaration: `source` is the enum's
//! own, and `Display` is forwarded to the payload.

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct Io;

impl fmt::Display for Io {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("io")
    }
}

impl Error for Io {}

#[derive(Debug)]
pub struct Parse;

impl fmt::Display for Parse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("parse")
    }
}

impl Error for Parse {}

#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
pub enum Failure {
    Io(Io),
    Parse(Parse),
}

#[whichever::dispatch]
impl whichever::foreign::Error for Failure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[test]
fn display_is_forwarded_and_source_is_the_enums() {
    let failure: Failure = Parse.into();
    assert_eq!(failure.to_string(), "parse");
    assert!(Error::source(&failure).is_none());
}
