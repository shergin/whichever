//! `Box<dyn Trait>`: the dynamic dispatch an enum replaces. The trait is the
//! hand-written family's, so the payload methods behind the vtable are the
//! very functions the enums call.
//!
//! `Boxed<V>` draws its variants from the first `V` payloads, like an enum of
//! `V` variants, so the two can be compared at every variant count. A
//! `Vec<&dyn Trait>` is not a separate subject: it loads the same fat pointer
//! and vtable slot and differs only in who owns the allocation.

macro_rules! boxed_dispatch {
    ($Trait:ident ; $($P:ident : $k:literal => $name:tt => $area:tt),* $(,)?) => {
        impl<const V: usize> crate::Dispatch for Boxed<V> {
            const SUBJECT: &'static str = "box_dyn";
            const VARIANTS: usize = V;

            #[inline]
            fn make(variant: usize, a: u64, b: u64) -> Self {
                // A payload's `K` is its position plus one.
                match variant % V + 1 {
                    $( $k => Boxed(Box::new($P::new(a, b))), )*
                    _ => unreachable!(),
                }
            }

            #[inline(always)]
            fn area(&self) -> u64 {
                self.0.area()
            }

            #[inline(always)]
            fn scale(&mut self, factor: u64) {
                self.0.scale(factor)
            }

            #[inline(always)]
            fn name(&self) -> &'static str {
                self.0.name()
            }
        }
    };
}

macro_rules! boxed_family {
    ($Trait:ident) => {
        pub struct Boxed<const V: usize>(pub Box<dyn $Trait>);

        with_payloads!(boxed_dispatch! { $Trait ; });

        pub type Any2 = Boxed<2>;
        pub type Any4 = Boxed<4>;
        pub type Any8 = Boxed<8>;
        pub type Any16 = Boxed<16>;
        pub type Any32 = Boxed<32>;
    };
}

pub mod trivial {
    use crate::hand::trivial::HandTrivial as Shape;
    use crate::payloads::*;

    boxed_family!(Shape);
}

pub mod opaque {
    use crate::hand::opaque::HandOpaque as Shape;
    use crate::payloads::*;

    boxed_family!(Shape);
}
