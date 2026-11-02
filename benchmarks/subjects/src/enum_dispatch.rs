//! `enum_dispatch` 0.3: `#[enum_dispatch(Enum, ..)]` on the trait names every
//! enum, `#[enum_dispatch]` on each enum. The link between the two goes through
//! a process-wide cache keyed by bare identifier, which is why every name here
//! is unique in the crate.

pub mod trivial {
    use crate::payloads::*;

    family! {
        subject: "enum_dispatch",
        inline: [],
        resize: yes,
        extra: no_extra,
        trait EnumDispatchTrivial {
            #[::enum_dispatch::enum_dispatch(
                EnumDispatchTrivial2,
                EnumDispatchTrivial4,
                EnumDispatchTrivial8,
                EnumDispatchTrivial16,
                EnumDispatchTrivial32
            )]
        }
        enums [EnumDispatchTrivial2, EnumDispatchTrivial4, EnumDispatchTrivial8, EnumDispatchTrivial16, EnumDispatchTrivial32] {
            #[::enum_dispatch::enum_dispatch]
        }
    }
}

pub mod opaque {
    use crate::payloads::*;

    family! {
        subject: "enum_dispatch",
        inline: [(never)],
        resize: yes,
        extra: no_extra,
        trait EnumDispatchOpaque {
            #[::enum_dispatch::enum_dispatch(
                EnumDispatchOpaque2,
                EnumDispatchOpaque4,
                EnumDispatchOpaque8,
                EnumDispatchOpaque16,
                EnumDispatchOpaque32
            )]
        }
        enums [EnumDispatchOpaque2, EnumDispatchOpaque4, EnumDispatchOpaque8, EnumDispatchOpaque16, EnumDispatchOpaque32] {
            #[::enum_dispatch::enum_dispatch]
        }
    }
}
