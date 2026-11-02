//! `ambassador` 0.5: `#[delegatable_trait]` on the trait,
//! `#[derive(Delegate)] #[delegate(Trait)]` on the enum. It cannot forward a
//! method returning `Self`, so this family has no `scaled`.

pub mod trivial {
    use crate::payloads::*;

    family! {
        subject: "ambassador",
        inline: [],
        resize: no,
        extra: no_extra,
        trait AmbassadorTrivial {
            #[::ambassador::delegatable_trait]
        }
        enums [AmbassadorTrivial2, AmbassadorTrivial4, AmbassadorTrivial8, AmbassadorTrivial16, AmbassadorTrivial32] {
            #[derive(::ambassador::Delegate)]
            #[delegate(AmbassadorTrivial)]
        }
    }
}

pub mod opaque {
    use crate::payloads::*;

    family! {
        subject: "ambassador",
        inline: [(never)],
        resize: no,
        extra: no_extra,
        trait AmbassadorOpaque {
            #[::ambassador::delegatable_trait]
        }
        enums [AmbassadorOpaque2, AmbassadorOpaque4, AmbassadorOpaque8, AmbassadorOpaque16, AmbassadorOpaque32] {
            #[derive(::ambassador::Delegate)]
            #[delegate(AmbassadorOpaque)]
        }
    }
}
