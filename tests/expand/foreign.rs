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

#[whichever::dispatchable(remote = io::Write)]
trait Write {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
}

#[whichever::dispatch(Write)]
pub enum Stream {
    Tcp(Tcp),
    #[cfg(unix)]
    Memory(Memory),
}

#[derive(Debug)]
#[whichever::dispatch(whichever::foreign::Display)]
pub enum ClientError {
    Io(io::Error),
    #[dispatch(from = false)]
    Message(String),
}

fn main() {}
