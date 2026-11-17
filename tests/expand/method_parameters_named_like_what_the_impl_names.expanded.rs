//! A method's generic parameter named like something the impl gives a
//! meaning, a parameter of the enum or a payload's type, is renamed with the
//! first number that neither the impl nor the trait writes, wherever the
//! signature names it, and in the trait method's `mismatch` expression.
pub trait Fill {
    fn get<'a>(&'a self, other: &'a u8) -> &'a u8;
    fn fill<const N: usize>(&self, with: [u8; N]) -> [u8; N];
    fn make<T: Default>(&self) -> T;
    fn pick<T: Copy, N>(&self, one: T, other: N) -> T
    where
        N: Into<T>;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Fill_de0dfdd3531393cd_5_1 as Fill;
pub struct Plain(pub u8);
impl Fill for Plain {
    fn get<'a>(&'a self, other: &'a u8) -> &'a u8 {
        other
    }
    fn make<T: Default>(&self) -> T {
        T::default()
    }
    fn fill<const N: usize>(&self, with: [u8; N]) -> [u8; N] {
        with
    }
    fn pick<T: Copy, N>(&self, one: T, _other: N) -> T
    where
        N: Into<T>,
    {
        one
    }
}
pub struct Slice<'s, T, const L: usize>(pub &'s [T; L]);
impl<T, const L: usize> Fill for Slice<'_, T, L> {
    fn get<'a>(&'a self, other: &'a u8) -> &'a u8 {
        other
    }
    fn make<U: Default>(&self) -> U {
        U::default()
    }
    fn fill<const N: usize>(&self, with: [u8; N]) -> [u8; N] {
        with
    }
    fn pick<U: Copy, N>(&self, one: U, _other: N) -> U
    where
        N: Into<U>,
    {
        one
    }
}
pub struct T2(pub u8);
pub trait Pick {
    fn pick<Plain: Copy>(&self, one: Plain) -> Plain;
    fn swap<T: Copy>(&self, one: T, other: T2) -> T;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Pick_dc1ee20ef1aae6b5_57_1 as Pick;
impl Pick for Plain {
    fn pick<P: Copy>(&self, one: P) -> P {
        one
    }
    fn swap<U: Copy>(&self, one: U, _other: T2) -> U {
        one
    }
}
impl<T, const L: usize> Pick for Slice<'_, T, L> {
    fn pick<P: Copy>(&self, one: P) -> P {
        one
    }
    fn swap<U: Copy>(&self, one: U, _other: T2) -> U {
        one
    }
}
pub trait Meet {
    fn meet<T: Default>(&self, other: &Self) -> T;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Meet_115bb7a094ecb0f3_81_1 as Meet;
impl Meet for Plain {
    fn meet<U: Default>(&self, _other: &Self) -> U {
        U::default()
    }
}
impl<T, const L: usize> Meet for Slice<'_, T, L> {
    fn meet<U: Default>(&self, _other: &Self) -> U {
        U::default()
    }
}
pub enum Holder<'a, T, const N: usize> {
    Plain(Plain),
    Slice(Slice<'a, T, N>),
}
#[automatically_derived]
impl<'a, T, const N: usize> ::core::convert::From<Plain> for Holder<'a, T, N> {
    #[inline]
    fn from(__value: Plain) -> Self {
        Self::Plain(__value)
    }
}
#[automatically_derived]
impl<'a, T, const N: usize> ::core::convert::From<Slice<'a, T, N>> for Holder<'a, T, N> {
    #[inline]
    fn from(__value: Slice<'a, T, N>) -> Self {
        Self::Slice(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Holder_a134d3745677ce7d_99_1 as Holder;
#[automatically_derived]
impl<'a, T, const N: usize> Fill for Holder<'a, T, N> {
    #[inline]
    fn get<'a2>(&'a2 self, other: &'a2 u8) -> &'a2 u8 {
        match self {
            Holder::Plain(__inner) => <Plain as Fill>::get(__inner, other),
            Holder::Slice(__inner) => <Slice<'a, T, N> as Fill>::get(__inner, other),
        }
    }
    #[inline]
    fn fill<const N2: usize>(&self, with: [u8; N2]) -> [u8; N2] {
        match self {
            Holder::Plain(__inner) => <Plain as Fill>::fill(__inner, with),
            Holder::Slice(__inner) => <Slice<'a, T, N> as Fill>::fill(__inner, with),
        }
    }
    #[inline]
    fn make<T2: Default>(&self) -> T2 {
        match self {
            Holder::Plain(__inner) => <Plain as Fill>::make::<T2>(__inner),
            Holder::Slice(__inner) => <Slice<'a, T, N> as Fill>::make::<T2>(__inner),
        }
    }
    #[inline]
    fn pick<T2: Copy, N2>(&self, one: T2, other: N2) -> T2
    where
        N2: Into<T2>,
    {
        match self {
            Holder::Plain(__inner) => <Plain as Fill>::pick(__inner, one, other),
            Holder::Slice(__inner) => {
                <Slice<'a, T, N> as Fill>::pick(__inner, one, other)
            }
        }
    }
}
#[automatically_derived]
impl<'a, T, const N: usize> Pick for Holder<'a, T, N> {
    #[inline]
    fn pick<Plain2: Copy>(&self, one: Plain2) -> Plain2 {
        match self {
            Holder::Plain(__inner) => <Plain as Pick>::pick(__inner, one),
            Holder::Slice(__inner) => <Slice<'a, T, N> as Pick>::pick(__inner, one),
        }
    }
    #[inline]
    fn swap<T3: Copy>(&self, one: T3, other: T2) -> T3 {
        match self {
            Holder::Plain(__inner) => <Plain as Pick>::swap(__inner, one, other),
            Holder::Slice(__inner) => {
                <Slice<'a, T, N> as Pick>::swap(__inner, one, other)
            }
        }
    }
}
#[automatically_derived]
impl<'a, T, const N: usize> Meet for Holder<'a, T, N> {
    #[inline]
    fn meet<T2: Default>(&self, other: &Self) -> T2 {
        match (self, other) {
            (Holder::Plain(__inner), Holder::Plain(__same0)) => {
                <Plain as Meet>::meet::<T2>(__inner, __same0)
            }
            (Holder::Slice(__inner), Holder::Slice(__same0)) => {
                <Slice<'a, T, N> as Meet>::meet::<T2>(__inner, __same0)
            }
            _ => T2::default(),
        }
    }
}
fn main() {}
