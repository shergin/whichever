//! A partial block over an enum whose only variant in the build cannot be
//! forwarded to, for a method that matches a `Self` argument beside `self`:
//! the variant's sentence stands alone, with no warning about the method's
//! other argument, which no forwarding arm is left to use.
pub mod real {
    pub trait Probe {
        fn base(&self) -> u64;
        fn f(&self, other: Self, n: u64) -> u64;
    }
}

#[whichever::dispatchable(remote = real::Probe)]
trait Probe {
    fn base(&self) -> u64;
    fn f(&self, other: Self, n: u64) -> u64;
}

pub struct P0(pub u64);

impl real::Probe for P0 {
    fn base(&self) -> u64 {
        self.0
    }
    fn f(&self, other: Self, n: u64) -> u64 {
        self.0 + other.0 + n
    }
}

pub struct P1(pub u64);

impl real::Probe for P1 {
    fn base(&self) -> u64 {
        self.0
    }
    fn f(&self, other: Self, n: u64) -> u64 {
        self.0 + other.0 + n
    }
}

#[whichever::dispatch]
pub enum E {
    #[cfg(any())]
    A(P0),
    B(P1, u64),
}

#[whichever::dispatch(mismatch(f = 424_242))]
impl Probe for E {}

fn main() {}
