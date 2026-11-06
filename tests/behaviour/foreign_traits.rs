use std::fmt;
use std::io::{self, Read, Write};
use std::num::ParseIntError;

#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
enum ClientError {
    Io(io::Error),
    Parse(ParseIntError),
    #[dispatch(from = false)]
    Message(String),
}

// `String` is no `Error`, so the enum answers `source` itself; the
// block names the declaration and the impl is of the real trait.
#[whichever::dispatch]
impl whichever::foreign::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientError::Io(error) => Some(error),
            _ => None,
        }
    }
}

// Would conflict with a generated impl, so it proves `from = false`.
impl From<String> for ClientError {
    fn from(message: String) -> Self {
        ClientError::Message(format!("hand-written: {message}"))
    }
}

#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
#[whichever::dispatch(whichever::foreign::Error)]
enum ReadError {
    Io(io::Error),
    Parse(ParseIntError),
}

#[test]
fn display_and_error_forward_to_the_payload() {
    let parse: ClientError = "x".parse::<u32>().unwrap_err().into();
    assert_eq!(parse.to_string(), "invalid digit found in string");
    let io: ClientError = io::Error::other("offline").into();
    assert_eq!(io.to_string(), "offline");
    assert!(std::error::Error::source(&io).is_some());
    let message: ClientError = String::from("bad url").into();
    assert_eq!(message.to_string(), "hand-written: bad url");
    assert!(std::error::Error::source(&message).is_none());
    let read: ReadError = io::Error::other("offline").into();
    assert_eq!(read.to_string(), "offline");
    assert!(std::error::Error::source(&read).is_none());
}

#[derive(Debug)]
struct Never;

#[whichever::dispatch(whichever::foreign::Read)]
#[whichever::dispatch(whichever::foreign::Write)]
enum Stream {
    Memory(io::Cursor<Vec<u8>>),
    Empty(io::Empty),
    #[cfg(any())]
    Never(Never),
}

#[test]
fn io_traits_forward_the_provided_methods_too() {
    let mut stream: Stream = io::Cursor::new(Vec::new()).into();
    stream.write_all(b"hello ").unwrap();
    write!(stream, "{}", 42).unwrap();
    let Stream::Memory(cursor) = &mut stream else {
        unreachable!()
    };
    cursor.set_position(0);
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    assert_eq!(text, "hello 42");
    let mut exact = [0u8; 3];
    assert!(Stream::from(io::empty()).read_exact(&mut exact).is_err());
}

#[whichever::dispatch(whichever::foreign::Iterator)]
enum Numbers {
    Range(std::ops::Range<u32>),
    Owned(std::vec::IntoIter<u32>),
}

#[test]
fn iterator_forwards_and_the_adapters_still_work() {
    let range: Numbers = (1..4).into();
    assert_eq!(range.size_hint(), (3, Some(3)));
    assert_eq!(range.fold(String::new(), |s, n| format!("{s}{n}")), "123");
    let owned: Numbers = vec![5, 6, 7].into_iter().into();
    assert_eq!(owned.map(|n| n * 2).collect::<Vec<_>>(), [10, 12, 14]);
    assert_eq!(Numbers::from(1..4).last(), Some(3));
}

#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::PartialEq)]
enum Id {
    Number(u32),
    Name(String),
}

#[test]
fn partial_eq_compares_same_variants_and_never_different_ones() {
    assert_eq!(Id::Number(1), Id::Number(1));
    assert_ne!(Id::Number(1), Id::Number(2));
    assert_ne!(Id::Number(1), Id::Name(String::from("1")));
}

// A restatement coexists with the real trait's import in one module,
// and a generic method goes through.
#[whichever::dispatchable(remote = fmt::LowerHex)]
trait LowerHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

#[whichever::dispatchable(remote = std::hash::Hash)]
trait Hash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H);
}

#[whichever::dispatch(LowerHex)]
#[whichever::dispatch(Hash)]
enum Word {
    Short(u8),
    Long(u64),
}

#[test]
fn a_restated_trait_is_implemented_at_its_real_path() {
    use std::hash::{Hash, Hasher};
    assert_eq!(format!("{:x}", Word::Short(255)), "ff");
    let mut hasher = std::hash::DefaultHasher::new();
    Word::Long(7).hash(&mut hasher);
    let mut expected = std::hash::DefaultHasher::new();
    7u64.hash(&mut expected);
    assert_eq!(hasher.finish(), expected.finish());
}
