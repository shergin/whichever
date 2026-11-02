//! The macros that stamp out one *family* per dispatch strategy: a trait, its
//! implementation for every payload type, five enums of 2 to 32 variants and
//! the [`Dispatch`](crate::Dispatch) adapters the benchmarks drive.
//!
//! The strategy under test is whatever the caller puts in the two attribute
//! lists (and, for the hand-written and `delegate!` subjects, in the `extra`
//! callback). Everything else is generated from the same tokens for every
//! family, so the strategy is the only variable between two subjects.
//!
//! One constraint shapes `subject_enum!`: `enum_dispatch` spans the binding
//! of its generated `match` arms from the variant and the use from the call
//! site, so the enum's attribute and its variant list have to come out of the
//! same `macro_rules!` expansion, which is why every variant list is written
//! out next to the `enum` it belongs to rather than passed in.

/// Hands the payload list to a callback macro, so that the structs, every
/// family's trait impls and the `Box<dyn>` constructors come from one list.
///
/// Each entry is `Name: K => name => { |a, b| area }`. `K` is the payload's
/// position plus one and goes into every method body; the `area` body cycles
/// through eight shapes so that no two payloads compile to the same code and
/// the arms of a `match` cannot be folded into one select chain, as they
/// could if the arms differed only by a constant. Every second payload
/// overrides the trait's default `name`, so defaults and overrides mix in
/// every enum.
macro_rules! with_payloads {
    ($callback:ident ! { $($head:tt)* }) => {
        $callback! {
            $($head)*
            V0: 1 => _ => { |a, b| a.wrapping_mul(1).wrapping_add(b) },
            V1: 2 => "v1" => { |a, b| a ^ b.wrapping_add(2) },
            V2: 3 => _ => { |a, b| a.rotate_left(3).wrapping_add(b) },
            V3: 4 => "v3" => { |a, b| (a | 4).wrapping_sub(b) },
            V4: 5 => _ => { |a, b| a.wrapping_add(b).wrapping_mul(5) },
            V5: 6 => "v5" => { |a, b| a.wrapping_mul(b).wrapping_add(6) },
            V6: 7 => _ => { |a, b| (a >> 1).wrapping_add(b) },
            V7: 8 => "v7" => { |a, b| a.wrapping_mul(8) ^ b },
            V8: 9 => _ => { |a, b| a.wrapping_mul(9).wrapping_add(b) },
            V9: 10 => "v9" => { |a, b| a ^ b.wrapping_add(10) },
            V10: 11 => _ => { |a, b| a.rotate_left(11).wrapping_add(b) },
            V11: 12 => "v11" => { |a, b| (a | 12).wrapping_sub(b) },
            V12: 13 => _ => { |a, b| a.wrapping_add(b).wrapping_mul(13) },
            V13: 14 => "v13" => { |a, b| a.wrapping_mul(b).wrapping_add(14) },
            V14: 15 => _ => { |a, b| (a >> 2).wrapping_add(b) },
            V15: 16 => "v15" => { |a, b| a.wrapping_mul(16) ^ b },
            V16: 17 => _ => { |a, b| a.wrapping_mul(17).wrapping_add(b) },
            V17: 18 => "v17" => { |a, b| a ^ b.wrapping_add(18) },
            V18: 19 => _ => { |a, b| a.rotate_left(19).wrapping_add(b) },
            V19: 20 => "v19" => { |a, b| (a | 20).wrapping_sub(b) },
            V20: 21 => _ => { |a, b| a.wrapping_add(b).wrapping_mul(21) },
            V21: 22 => "v21" => { |a, b| a.wrapping_mul(b).wrapping_add(22) },
            V22: 23 => _ => { |a, b| (a >> 3).wrapping_add(b) },
            V23: 24 => "v23" => { |a, b| a.wrapping_mul(24) ^ b },
            V24: 25 => _ => { |a, b| a.wrapping_mul(25).wrapping_add(b) },
            V25: 26 => "v25" => { |a, b| a ^ b.wrapping_add(26) },
            V26: 27 => _ => { |a, b| a.rotate_left(27).wrapping_add(b) },
            V27: 28 => "v27" => { |a, b| (a | 28).wrapping_sub(b) },
            V28: 29 => _ => { |a, b| a.wrapping_add(b).wrapping_mul(29) },
            V29: 30 => "v29" => { |a, b| a.wrapping_mul(b).wrapping_add(30) },
            V30: 31 => _ => { |a, b| (a >> 4).wrapping_add(b) },
            V31: 32 => "v31" => { |a, b| a.wrapping_mul(32) ^ b },
        }
    };
}

/// The trait of a family. `yes` adds the `-> Self` method, which not every
/// strategy can forward; `Self: Sized` on it keeps the trait object safe for
/// the `Box<dyn>` subject.
macro_rules! define_trait {
    (yes, $Trait:ident { $(#[$attr:meta])* }) => {
        $(#[$attr])*
        pub trait $Trait {
            fn area(&self) -> u64;
            fn scale(&mut self, factor: u64);
            fn name(&self) -> &'static str {
                "shape"
            }
            fn scaled(&self, factor: u64) -> Self
            where
                Self: Sized;
        }
    };
    (no, $Trait:ident { $(#[$attr:meta])* }) => {
        $(#[$attr])*
        pub trait $Trait {
            fn area(&self) -> u64;
            fn scale(&mut self, factor: u64);
            fn name(&self) -> &'static str {
                "shape"
            }
        }
    };
}

/// Implements a family's trait for every payload. `$inline` is a bracket
/// group that goes after `inline` on every method: `[]` for the `trivial`
/// family, `[(never)]` for `opaque`.
macro_rules! payload_impls {
    (
        $Trait:ident, $inline:tt, $resize:tt ;
        $($P:ident : $k:literal => $name:tt => $area:tt),* $(,)?
    ) => {
        $( payload_impl!($Trait, $inline, $resize, $P, $k, $name, $area); )*
    };
}

macro_rules! payload_impl {
    ($Trait:ident, [$($inline:tt)*], $resize:tt, $P:ident, $k:literal, _, $area:tt) => {
        impl $Trait for $P {
            payload_methods!([$($inline)*], $resize, $k, $area);
        }
    };
    ($Trait:ident, [$($inline:tt)*], $resize:tt, $P:ident, $k:literal, $name:literal, $area:tt) => {
        impl $Trait for $P {
            payload_methods!([$($inline)*], $resize, $k, $area);
            #[inline $($inline)*]
            fn name(&self) -> &'static str {
                $name
            }
        }
    };
}

/// The `area` body is written as `|a, b| expr` in the payload list, so that
/// the names it uses and the `let`s that bind them share one macro context.
macro_rules! payload_methods {
    ([$($inline:tt)*], yes, $k:literal, $area:tt) => {
        payload_methods!([$($inline)*], no, $k, $area);
        #[inline $($inline)*]
        fn scaled(&self, factor: u64) -> Self {
            Self {
                a: self.a.wrapping_mul(factor),
                b: self.b.wrapping_add($k),
            }
        }
    };
    ([$($inline:tt)*], no, $k:literal, { |$a:ident, $b:ident| $area:expr }) => {
        #[inline $($inline)*]
        fn area(&self) -> u64 {
            let $a = self.a;
            let $b = self.b;
            $area
        }
        #[inline $($inline)*]
        fn scale(&mut self, factor: u64) {
            self.a = self.a.wrapping_mul(factor).wrapping_add($k);
        }
    };
}

/// One enum of a family, with the strategy's attributes, plus its adapter.
/// The variant lists are written out, see the module docs.
macro_rules! subject_enum {
    ($subject:literal, $Trait:ident, $resize:tt, $extra:ident, { $(#[$attr:meta])* } $E:ident = 2) => {
        $(#[$attr])*
        pub enum $E {
            V0(V0), V1(V1),
        }
        dispatch_adapter!($subject, $Trait, $resize, $extra, $E = 2 [V0 = 0, V1 = 1]);
    };
    ($subject:literal, $Trait:ident, $resize:tt, $extra:ident, { $(#[$attr:meta])* } $E:ident = 4) => {
        $(#[$attr])*
        pub enum $E {
            V0(V0), V1(V1), V2(V2), V3(V3),
        }
        dispatch_adapter!($subject, $Trait, $resize, $extra, $E = 4 [
            V0 = 0, V1 = 1, V2 = 2, V3 = 3,
        ]);
    };
    ($subject:literal, $Trait:ident, $resize:tt, $extra:ident, { $(#[$attr:meta])* } $E:ident = 8) => {
        $(#[$attr])*
        pub enum $E {
            V0(V0), V1(V1), V2(V2), V3(V3), V4(V4), V5(V5), V6(V6), V7(V7),
        }
        dispatch_adapter!($subject, $Trait, $resize, $extra, $E = 8 [
            V0 = 0, V1 = 1, V2 = 2, V3 = 3, V4 = 4, V5 = 5, V6 = 6, V7 = 7,
        ]);
    };
    ($subject:literal, $Trait:ident, $resize:tt, $extra:ident, { $(#[$attr:meta])* } $E:ident = 16) => {
        $(#[$attr])*
        pub enum $E {
            V0(V0), V1(V1), V2(V2), V3(V3), V4(V4), V5(V5), V6(V6), V7(V7),
            V8(V8), V9(V9), V10(V10), V11(V11), V12(V12), V13(V13), V14(V14), V15(V15),
        }
        dispatch_adapter!($subject, $Trait, $resize, $extra, $E = 16 [
            V0 = 0, V1 = 1, V2 = 2, V3 = 3, V4 = 4, V5 = 5, V6 = 6, V7 = 7,
            V8 = 8, V9 = 9, V10 = 10, V11 = 11, V12 = 12, V13 = 13, V14 = 14, V15 = 15,
        ]);
    };
    ($subject:literal, $Trait:ident, $resize:tt, $extra:ident, { $(#[$attr:meta])* } $E:ident = 32) => {
        $(#[$attr])*
        pub enum $E {
            V0(V0), V1(V1), V2(V2), V3(V3), V4(V4), V5(V5), V6(V6), V7(V7),
            V8(V8), V9(V9), V10(V10), V11(V11), V12(V12), V13(V13), V14(V14), V15(V15),
            V16(V16), V17(V17), V18(V18), V19(V19), V20(V20), V21(V21), V22(V22), V23(V23),
            V24(V24), V25(V25), V26(V26), V27(V27), V28(V28), V29(V29), V30(V30), V31(V31),
        }
        dispatch_adapter!($subject, $Trait, $resize, $extra, $E = 32 [
            V0 = 0, V1 = 1, V2 = 2, V3 = 3, V4 = 4, V5 = 5, V6 = 6, V7 = 7,
            V8 = 8, V9 = 9, V10 = 10, V11 = 11, V12 = 12, V13 = 13, V14 = 14, V15 = 15,
            V16 = 16, V17 = 17, V18 = 18, V19 = 19, V20 = 20, V21 = 21, V22 = 22, V23 = 23,
            V24 = 24, V25 = 25, V26 = 26, V27 = 27, V28 = 28, V29 = 29, V30 = 30, V31 = 31,
        ]);
    };
}

/// The `Dispatch` adapter of one enum. Its methods are `#[inline(always)]`,
/// so what the benchmark loop sees is exactly the strategy's own method, with
/// whatever inlining that method allows.
macro_rules! dispatch_adapter {
    (
        $subject:literal, $Trait:ident, $resize:tt, $extra:ident,
        $E:ident = $n:literal [ $($V:ident = $i:literal),* $(,)? ]
    ) => {
        impl crate::Dispatch for $E {
            const SUBJECT: &'static str = $subject;
            const VARIANTS: usize = $n;

            #[inline]
            fn make(variant: usize, a: u64, b: u64) -> Self {
                match variant % $n {
                    $( $i => $E::$V($V::new(a, b)), )*
                    _ => unreachable!(),
                }
            }

            #[inline(always)]
            fn area(&self) -> u64 {
                <$E as $Trait>::area(self)
            }

            #[inline(always)]
            fn scale(&mut self, factor: u64) {
                <$E as $Trait>::scale(self, factor)
            }

            #[inline(always)]
            fn name(&self) -> &'static str {
                <$E as $Trait>::name(self)
            }
        }

        resize_adapter!($resize, $E, $Trait);
        $extra!($Trait, $E, $resize, [ $($V = $i),* ]);
    };
}

macro_rules! resize_adapter {
    (yes, $E:ident, $Trait:ident) => {
        impl crate::Resize for $E {
            #[inline(always)]
            fn scaled(&self, factor: u64) -> Self {
                <$E as $Trait>::scaled(self, factor)
            }
        }
    };
    (no, $E:ident, $Trait:ident) => {};
}

/// A whole family: the trait, its payload impls, the five enums and the
/// `Any2`..`Any32` aliases every family exports under the same names.
///
/// Every trait and enum needs a name that is unique in this crate:
/// `enum_dispatch` keeps a process-wide cache keyed by bare identifier and
/// `ambassador` exports a macro named after the trait at the crate root.
macro_rules! family {
    (
        subject: $subject:literal,
        inline: $inline:tt,
        resize: $resize:tt,
        extra: $extra:ident,
        trait $Trait:ident { $(#[$tattr:meta])* }
        enums [$E2:ident, $E4:ident, $E8:ident, $E16:ident, $E32:ident] { $(#[$eattr:meta])* }
    ) => {
        define_trait!($resize, $Trait { $(#[$tattr])* });
        with_payloads!(payload_impls! { $Trait, $inline, $resize ; });

        subject_enum!($subject, $Trait, $resize, $extra, { $(#[$eattr])* } $E2 = 2);
        subject_enum!($subject, $Trait, $resize, $extra, { $(#[$eattr])* } $E4 = 4);
        subject_enum!($subject, $Trait, $resize, $extra, { $(#[$eattr])* } $E8 = 8);
        subject_enum!($subject, $Trait, $resize, $extra, { $(#[$eattr])* } $E16 = 16);
        subject_enum!($subject, $Trait, $resize, $extra, { $(#[$eattr])* } $E32 = 32);

        pub type Any2 = $E2;
        pub type Any4 = $E4;
        pub type Any8 = $E8;
        pub type Any16 = $E16;
        pub type Any32 = $E32;
    };
}

/// The `extra` callback of the strategies whose impl comes from an attribute.
macro_rules! no_extra {
    ($($tokens:tt)*) => {};
}

/// The `extra` callback of the hand-written subject: the impl a careful hand
/// writes, and the one `whichever` claims to reproduce.
macro_rules! hand_impl {
    ($Trait:ident, $E:ident, yes, [ $($V:ident = $i:literal),* ]) => {
        impl $Trait for $E {
            #[inline]
            fn area(&self) -> u64 {
                match self {
                    $( $E::$V(inner) => <$V as $Trait>::area(inner), )*
                }
            }
            #[inline]
            fn scale(&mut self, factor: u64) {
                match self {
                    $( $E::$V(inner) => <$V as $Trait>::scale(inner, factor), )*
                }
            }
            #[inline]
            fn name(&self) -> &'static str {
                match self {
                    $( $E::$V(inner) => <$V as $Trait>::name(inner), )*
                }
            }
            #[inline]
            fn scaled(&self, factor: u64) -> Self {
                match self {
                    $( $E::$V(inner) => $E::$V(<$V as $Trait>::scaled(inner, factor)), )*
                }
            }
        }
    };
}

/// The `extra` callback of the `delegate` subject. `delegate!` is not an
/// attribute: the signatures are listed again inside the impl, and `-> Self`
/// needs `#[into]` plus a `From` impl per payload.
macro_rules! delegate_impl {
    ($Trait:ident, $E:ident, yes, [ $($V:ident = $i:literal),* ]) => {
        $(
            impl ::core::convert::From<$V> for $E {
                #[inline]
                fn from(value: $V) -> Self {
                    $E::$V(value)
                }
            }
        )*

        impl $Trait for $E {
            ::delegate::delegate! {
                to match self {
                    $( $E::$V(inner) => inner, )*
                } {
                    fn area(&self) -> u64;
                    fn scale(&mut self, factor: u64);
                    fn name(&self) -> &'static str;
                    #[into]
                    fn scaled(&self, factor: u64) -> Self;
                }
            }
        }
    };
}
