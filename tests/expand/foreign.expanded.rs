use std::io;
pub struct Tcp(pub Vec<u8>);
pub struct Memory(pub Vec<u8>);
impl io::Write for Tcp {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl io::Write for Memory {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
use __whichever_Write_1892d5bed05f318e_26_1 as Write;
pub enum Stream {
    Tcp(Tcp),
    Memory(Memory),
}
#[automatically_derived]
impl ::core::convert::From<Tcp> for Stream {
    #[inline]
    fn from(__value: Tcp) -> Self {
        Self::Tcp(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Memory> for Stream {
    #[inline]
    fn from(__value: Memory) -> Self {
        Self::Memory(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Stream_2b72aa00a652485e_33_1 as Stream;
#[automatically_derived]
impl io::Write for Stream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Tcp(__inner) => <Tcp as io::Write>::write(__inner, buf),
            Stream::Memory(__inner) => <Memory as io::Write>::write(__inner, buf),
        }
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Tcp(__inner) => <Tcp as io::Write>::flush(__inner),
            Stream::Memory(__inner) => <Memory as io::Write>::flush(__inner),
        }
    }
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Stream::Tcp(__inner) => <Tcp as io::Write>::write_all(__inner, buf),
            Stream::Memory(__inner) => <Memory as io::Write>::write_all(__inner, buf),
        }
    }
}
pub enum ClientError {
    Io(io::Error),
    Message(String),
}
#[automatically_derived]
impl ::core::convert::From<io::Error> for ClientError {
    #[inline]
    fn from(__value: io::Error) -> Self {
        Self::Io(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_ClientError_97504760822228e9_41_1 as ClientError;
#[automatically_derived]
impl ::core::fmt::Display for ClientError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
            ClientError::Io(__inner) => {
                <io::Error as ::core::fmt::Display>::fmt(__inner, f)
            }
            ClientError::Message(__inner) => {
                <String as ::core::fmt::Display>::fmt(__inner, f)
            }
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for ClientError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Self::Io(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Io", &__self_0)
            }
            Self::Message(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "Message",
                    &__self_0,
                )
            }
        }
    }
}
fn main() {}
