//! A generic method is called with its parameters spelled out only when no
//! argument fixes them.
use std::str::FromStr;
pub trait Source {
    fn parse<T: FromStr>(&self) -> Option<T>;
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32;
    fn zeros<const N: usize>(&self) -> [u8; N];
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Source_7f1fae81fd5f9de2_5_1 as Source;
pub struct Text(pub &'static str);
pub struct Number(pub u32);
impl Source for Text {
    fn parse<T: FromStr>(&self) -> Option<T> {
        self.0.parse().ok()
    }
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
        f(self.0.len() as u32)
    }
    fn zeros<const N: usize>(&self) -> [u8; N] {
        [0; N]
    }
}
impl Source for Number {
    fn parse<T: FromStr>(&self) -> Option<T> {
        self.0.to_string().parse().ok()
    }
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
        f(self.0)
    }
    fn zeros<const N: usize>(&self) -> [u8; N] {
        [0; N]
    }
}
pub enum Any {
    Text(Text),
    Number(Number),
}
#[automatically_derived]
impl ::core::convert::From<Text> for Any {
    #[inline]
    fn from(__value: Text) -> Self {
        Self::Text(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Number> for Any {
    #[inline]
    fn from(__value: Number) -> Self {
        Self::Number(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Any_c6695dac96144106_39_1 as Any;
#[automatically_derived]
impl Source for Any {
    #[inline]
    fn parse<T: FromStr>(&self) -> Option<T> {
        match self {
            Any::Text(__inner) => <Text as Source>::parse::<T>(__inner),
            Any::Number(__inner) => <Number as Source>::parse::<T>(__inner),
        }
    }
    #[inline]
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
        match self {
            Any::Text(__inner) => <Text as Source>::apply(__inner, f),
            Any::Number(__inner) => <Number as Source>::apply(__inner, f),
        }
    }
    #[inline]
    fn zeros<const N: usize>(&self) -> [u8; N] {
        match self {
            Any::Text(__inner) => <Text as Source>::zeros::<N>(__inner),
            Any::Number(__inner) => <Number as Source>::zeros::<N>(__inner),
        }
    }
}
fn main() {}
