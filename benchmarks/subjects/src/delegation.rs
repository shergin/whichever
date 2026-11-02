//! `delegation` 0.5: `#[delegate]` on the trait, `#[delegate(derive(Trait))]`
//! on the enum. Rather than a `match`, it transmutes the enum into a wrapper
//! and converts it to a nested `Either` chain that the wrapper's impl walks;
//! the generated methods carry no `#[inline]`. It cannot forward a method
//! returning `Self`, so this family has no `scaled`.

pub mod trivial {
    use crate::payloads::*;

    family! {
        subject: "delegation",
        inline: [],
        resize: no,
        extra: no_extra,
        trait DelegationTrivial {
            #[::delegation::delegate]
        }
        enums [DelegationTrivial2, DelegationTrivial4, DelegationTrivial8, DelegationTrivial16, DelegationTrivial32] {
            #[::delegation::delegate(derive(DelegationTrivial))]
        }
    }
}

pub mod opaque {
    use crate::payloads::*;

    family! {
        subject: "delegation",
        inline: [(never)],
        resize: no,
        extra: no_extra,
        trait DelegationOpaque {
            #[::delegation::delegate]
        }
        enums [DelegationOpaque2, DelegationOpaque4, DelegationOpaque8, DelegationOpaque16, DelegationOpaque32] {
            #[::delegation::delegate(derive(DelegationOpaque))]
        }
    }
}
