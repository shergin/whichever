//! A trait whose carrier this checkout writes, with a provided method
//! without a receiver, which the signatures-only carrier names in
//! `provided` and an older reader cannot see.

/// A device.
#[whichever::dispatchable]
pub trait Device {
    /// Returns the identifier.
    fn id(&self) -> u32;
    /// Returns the known kinds.
    fn kinds() -> &'static [&'static str] {
        &["device"]
    }
}
