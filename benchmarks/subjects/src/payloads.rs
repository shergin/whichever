//! The 32 payload types every enum is built from.
//!
//! All payloads have the same layout, two `u64`s, so an enum is 24 bytes at
//! every variant count and the only thing that changes between `Any2` and
//! `Any32` is the number of arms. Each payload's methods have a body of their
//! own, a couple of integer operations, so no two payloads compile to the
//! same code; the bodies are in the list in `macros.rs`.

macro_rules! payload_structs {
    ($($P:ident : $k:literal => $name:tt => $area:tt),* $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $P {
                pub a: u64,
                pub b: u64,
            }

            impl $P {
                /// This payload's position in the list plus one.
                pub const K: u64 = $k;

                #[inline]
                pub const fn new(a: u64, b: u64) -> Self {
                    Self { a, b }
                }
            }
        )*
    };
}

with_payloads!(payload_structs! {});
