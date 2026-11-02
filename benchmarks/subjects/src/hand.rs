//! Hand-written forwarding: `match self`, one fully qualified call per arm,
//! `#[inline]` on every method. The reference every other subject is
//! measured against, and the code `whichever` claims to reproduce.

pub mod trivial {
    use crate::payloads::*;

    family! {
        subject: "hand",
        inline: [],
        resize: yes,
        extra: hand_impl,
        trait HandTrivial {}
        enums [HandTrivial2, HandTrivial4, HandTrivial8, HandTrivial16, HandTrivial32] {}
    }
}

pub mod opaque {
    use crate::payloads::*;

    family! {
        subject: "hand",
        inline: [(never)],
        resize: yes,
        extra: hand_impl,
        trait HandOpaque {}
        enums [HandOpaque2, HandOpaque4, HandOpaque8, HandOpaque16, HandOpaque32] {}
    }
}
