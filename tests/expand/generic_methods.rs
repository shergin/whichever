//! A generic method is called with its parameters spelled out only when no
//! argument fixes them.
use std::str::FromStr;

#[whichever::dispatchable]
pub trait Source {
    fn parse<T: FromStr>(&self) -> Option<T>;
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32;
    fn zeros<const N: usize>(&self) -> [u8; N];
}

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

#[whichever::dispatch(Source)]
pub enum Any {
    Text(Text),
    Number(Number),
}

fn main() {}
