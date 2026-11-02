//! Implements a trait for an enum by forwarding to whichever variant it holds.
//!
//! `#[dispatchable]` goes on the trait, in the crate that owns it.
//! `#[dispatch(Trait)]` goes on an enum whose variants each hold one value
//! implementing the trait, in any crate. The enum then implements the trait
//! with one `match` per method, each arm a fully qualified call into the
//! variant's payload, and gets a `From<Payload>` impl per variant.
//!
//! Options on `#[dispatch(..)]`, after the trait path:
//! - `Name = Type` fixes an associated type instead of taking it from the
//!   first variant; a `Result<_, Self::Name>` return is then converted with
//!   `Into`.
//! - `mismatch(method = expr)` gives the result of `method` when its `Self`
//!   arguments hold a different variant than `self`; the trait author can
//!   set it instead with `#[dispatch(mismatch = expr)]` on the method.
//! - `from = false` suppresses the `From` impls.
pub use whichever_macros::{dispatch, dispatchable};

#[doc(hidden)]
pub mod __private {
    pub use whichever_macros::implement;
}
