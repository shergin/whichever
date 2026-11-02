//! No dispatch at all: a collection of one concrete payload type, calling its
//! method directly. The floor every strategy is measured against; it does not
//! change with the variant count, so every `AnyN` alias is the same type.

macro_rules! mono_family {
    ($Trait:ident) => {
        #[repr(transparent)]
        pub struct Mono(pub V0);

        impl crate::Dispatch for Mono {
            const SUBJECT: &'static str = "monomorphic";
            const VARIANTS: usize = 1;

            #[inline]
            fn make(_variant: usize, a: u64, b: u64) -> Self {
                Mono(V0::new(a, b))
            }

            #[inline(always)]
            fn area(&self) -> u64 {
                <V0 as $Trait>::area(&self.0)
            }

            #[inline(always)]
            fn scale(&mut self, factor: u64) {
                <V0 as $Trait>::scale(&mut self.0, factor)
            }

            #[inline(always)]
            fn name(&self) -> &'static str {
                <V0 as $Trait>::name(&self.0)
            }
        }

        impl crate::Resize for Mono {
            #[inline(always)]
            fn scaled(&self, factor: u64) -> Self {
                Mono(<V0 as $Trait>::scaled(&self.0, factor))
            }
        }

        pub type Any2 = Mono;
        pub type Any4 = Mono;
        pub type Any8 = Mono;
        pub type Any16 = Mono;
        pub type Any32 = Mono;
    };
}

pub mod trivial {
    use crate::hand::trivial::HandTrivial as Shape;
    use crate::payloads::V0;

    mono_family!(Shape);
}

pub mod opaque {
    use crate::hand::opaque::HandOpaque as Shape;
    use crate::payloads::V0;

    mono_family!(Shape);
}
