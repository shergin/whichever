//! `whichever`: `#[dispatchable]` on the trait, `#[dispatch(Trait)]` on the
//! enum. The trait's tokens travel through a hidden exported macro, so the two
//! may live in different crates; here they share a module like every subject.

pub mod trivial {
    use crate::payloads::*;

    family! {
        subject: "whichever",
        inline: [],
        resize: yes,
        extra: no_extra,
        trait WhicheverTrivial {
            #[::whichever::dispatchable]
        }
        enums [WhicheverTrivial2, WhicheverTrivial4, WhicheverTrivial8, WhicheverTrivial16, WhicheverTrivial32] {
            #[::whichever::dispatch(WhicheverTrivial)]
        }
    }
}

pub mod opaque {
    use crate::payloads::*;

    family! {
        subject: "whichever",
        inline: [(never)],
        resize: yes,
        extra: no_extra,
        trait WhicheverOpaque {
            #[::whichever::dispatchable]
        }
        enums [WhicheverOpaque2, WhicheverOpaque4, WhicheverOpaque8, WhicheverOpaque16, WhicheverOpaque32] {
            #[::whichever::dispatch(WhicheverOpaque)]
        }
    }
}
