//! Two storage backends behind one enum: `cargo run --example storage`.

use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[whichever::dispatchable]
pub trait Storage: Sized {
    type Error;

    fn load(&self, key: &str) -> Result<Vec<u8>, Self::Error>;

    /// Two different backends never share a file.
    #[dispatch(mismatch = false)]
    fn shares_file_with(&self, other: &Self) -> bool;

    fn snapshot(&self) -> Option<Self>;
}

#[derive(Debug)]
#[whichever::dispatch(Storage, Error = StorageError)]
#[whichever::dispatch(whichever::foreign::Display)]
pub enum AnyStorage {
    Memory(MemoryStorage), // Error = Infallible
    Disk(DiskStorage),     // Error = io::Error
}

/// A backend that keeps every value in a map and never fails.
#[derive(Debug, Clone, Default)]
pub struct MemoryStorage {
    values: HashMap<String, Vec<u8>>,
}

impl Storage for MemoryStorage {
    type Error = Infallible;

    fn load(&self, key: &str) -> Result<Vec<u8>, Infallible> {
        Ok(self.values.get(key).cloned().unwrap_or_default())
    }

    fn shares_file_with(&self, _other: &Self) -> bool {
        false
    }

    fn snapshot(&self) -> Option<Self> {
        Some(self.clone())
    }
}

impl fmt::Display for MemoryStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "memory")
    }
}

/// A backend that reads one file per key from a directory.
#[derive(Debug)]
pub struct DiskStorage {
    directory: PathBuf,
}

impl Storage for DiskStorage {
    type Error = io::Error;

    fn load(&self, key: &str) -> Result<Vec<u8>, io::Error> {
        std::fs::read(self.directory.join(key))
    }

    fn shares_file_with(&self, other: &Self) -> bool {
        self.directory == other.directory
    }

    /// An open directory cannot be copied, so a disk backend has no snapshot.
    fn snapshot(&self) -> Option<Self> {
        None
    }
}

impl fmt::Display for DiskStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "disk at {}", self.directory.display())
    }
}

/// The one error type the enum reports, whichever backend failed.
#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
}

impl From<Infallible> for StorageError {
    fn from(never: Infallible) -> Self {
        match never {}
    }
}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        StorageError::Io(error)
    }
}

fn main() {
    let mut values = HashMap::new();
    values.insert("greeting".to_string(), b"hello".to_vec());
    let memory = AnyStorage::from(MemoryStorage { values });
    let disk = AnyStorage::from(DiskStorage {
        directory: PathBuf::from("/nonexistent/whichever-example"),
    });

    // Each backend's own error type arrives as `StorageError` through `Into`.
    assert_eq!(memory.load("greeting").unwrap(), b"hello");
    let error = disk.load("greeting").unwrap_err();
    assert!(matches!(error, StorageError::Io(ref io) if io.kind() == io::ErrorKind::NotFound));

    // Two different variants answer the `false` the trait declares.
    assert!(!memory.shares_file_with(&disk));
    assert!(disk.shares_file_with(&disk));

    // The payload's `Option<Self>` comes back wrapped in its own variant.
    assert!(matches!(memory.snapshot(), Some(AnyStorage::Memory(_))));
    assert!(disk.snapshot().is_none());

    // `Display` is a trait of `std`, forwarded to the payload's own impl.
    assert_eq!(memory.to_string(), "memory");
    assert_eq!(disk.to_string(), "disk at /nonexistent/whichever-example");

    println!("{memory}; {disk}; {error:?}");
}
