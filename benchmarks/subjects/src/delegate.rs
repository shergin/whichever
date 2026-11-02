//! `delegate` 0.13: the `delegate!` macro inside a hand-written impl block,
//! with `to match self { .. }` naming every variant and every signature listed
//! again. Half-way between the hand-written impl and an attribute.

pub mod trivial {
    use crate::payloads::*;

    family! {
        subject: "delegate",
        inline: [],
        resize: yes,
        extra: delegate_impl,
        trait DelegateTrivial {}
        enums [DelegateTrivial2, DelegateTrivial4, DelegateTrivial8, DelegateTrivial16, DelegateTrivial32] {}
    }
}

pub mod opaque {
    use crate::payloads::*;

    family! {
        subject: "delegate",
        inline: [(never)],
        resize: yes,
        extra: delegate_impl,
        trait DelegateOpaque {}
        enums [DelegateOpaque2, DelegateOpaque4, DelegateOpaque8, DelegateOpaque16, DelegateOpaque32] {}
    }
}
