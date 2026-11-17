//! A method's generic parameter named like something the impl gives a
//! meaning, a parameter of the enum or a payload's type, is renamed with the
//! first number that neither the impl nor the trait writes, wherever the
//! signature names it, and in the trait method's `mismatch` expression.
#[whichever::dispatchable]
pub trait Fill {
    fn get<'a>(&'a self, other: &'a u8) -> &'a u8;
    fn fill<const N: usize>(&self, with: [u8; N]) -> [u8; N];
    fn make<T: Default>(&self) -> T;
    fn pick<T: Copy, N>(&self, one: T, other: N) -> T
    where
        N: Into<T>;
}

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

#[whichever::dispatchable]
pub trait Pick {
    fn pick<Plain: Copy>(&self, one: Plain) -> Plain;
    fn swap<T: Copy>(&self, one: T, other: T2) -> T;
}

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

#[whichever::dispatchable]
pub trait Meet {
    #[dispatch(mismatch = T::default())]
    fn meet<T: Default>(&self, other: &Self) -> T;
}

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

#[whichever::dispatch(Fill)]
#[whichever::dispatch(Pick)]
#[whichever::dispatch(Meet)]
pub enum Holder<'a, T, const N: usize> {
    Plain(Plain),
    Slice(Slice<'a, T, N>),
}

fn main() {}
