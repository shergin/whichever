//! Traits with the shapes a dispatch enum meets in practice, defined in
//! their own crate so that `smoke-enums` dispatches them across a crate
//! boundary.
#![allow(async_fn_in_trait)]
#![deny(missing_docs)]
use std::fmt::Debug;

use whichever::dispatchable;

/// A short description of something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Description(pub &'static str);

/// A flat shape.
#[dispatchable]
pub trait Shape: Debug {
    /// Returns the area.
    fn area(&self) -> u64;

    /// Returns the kind of shape; the default is generic.
    fn name(&self) -> &'static str {
        "shape"
    }

    /// Returns whether both shapes have the same dimensions; two different
    /// kinds of shape never do.
    #[dispatch(mismatch = false)]
    fn congruent(&self, other: &Self) -> bool
    where
        Self: Sized;

    /// Returns this shape scaled by `factor`.
    fn scaled(&self, factor: u64) -> Self
    where
        Self: Sized;

    /// Returns a description. The type is named through `crate::` on
    /// purpose: the path must survive the move into the enum's crate.
    fn describe(&self) -> crate::Description {
        crate::Description("a shape")
    }
}

/// A key-value store with an error type of its own.
#[dispatchable]
pub trait Storage {
    /// The error of this store.
    type Error;

    /// Returns the value stored under `key`.
    fn load(&self, key: &str) -> Result<u32, Self::Error>;

    /// Writes pending changes out.
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

/// A serialisation format.
pub trait Format {
    /// The values this format carries.
    type Value;
}

/// Encodes the values of one format.
#[dispatchable]
pub trait Encoder<F: Format> {
    /// What encoding produces.
    type Output;

    /// Encodes one value.
    fn encode(&self, value: &F::Value) -> Self::Output;

    /// Encodes every value, in order.
    fn encode_all(&self, values: &[F::Value]) -> Vec<Self::Output> {
        values.iter().map(|value| self.encode(value)).collect()
    }
}

/// A peripheral device.
#[dispatchable]
pub trait Device: Debug + Sized {
    /// Returns the identifier.
    fn id(&self) -> &u32;

    /// Returns a copy with another identifier.
    fn with_id(&self, id: u32) -> Self;

    /// Returns the kind of device.
    fn kind(&self) -> &'static str {
        "device"
    }

    /// Prepares the device; nothing to do by default.
    async fn connect(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Applies `f` to the identifier.
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
        f(*self.id())
    }

    /// Returns the known kinds; a dispatch enum keeps this default.
    fn kinds() -> &'static [&'static str] {
        &["device"]
    }
}
