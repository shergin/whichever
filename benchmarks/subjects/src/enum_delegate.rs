//! `enum_delegate` 0.2: `#[register]` on the trait, `#[implement(Trait)]` on
//! the enum. Its generated methods carry no `#[inline]` and convert every
//! result with `.into()` and every argument with `.try_into()`.

pub mod trivial {
    use crate::payloads::*;

    family! {
        subject: "enum_delegate",
        inline: [],
        resize: yes,
        extra: no_extra,
        trait EnumDelegateTrivial {
            #[::enum_delegate::register]
        }
        enums [EnumDelegateTrivial2, EnumDelegateTrivial4, EnumDelegateTrivial8, EnumDelegateTrivial16, EnumDelegateTrivial32] {
            #[::enum_delegate::implement(EnumDelegateTrivial)]
        }
    }
}

pub mod opaque {
    use crate::payloads::*;

    family! {
        subject: "enum_delegate",
        inline: [(never)],
        resize: yes,
        extra: no_extra,
        trait EnumDelegateOpaque {
            #[::enum_delegate::register]
        }
        enums [EnumDelegateOpaque2, EnumDelegateOpaque4, EnumDelegateOpaque8, EnumDelegateOpaque16, EnumDelegateOpaque32] {
            #[::enum_delegate::implement(EnumDelegateOpaque)]
        }
    }
}
