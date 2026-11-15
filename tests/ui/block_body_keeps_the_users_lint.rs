//! A partial block's method that names a variant and has a body for the
//! rest is the user's, and so is the lint about its arguments: over an enum
//! whose every variant is under `#[cfg]`, the build that compiles them all
//! out has only the body, and a body that leaves an argument unused is
//! reported as the same impl written by hand is. The module denies the
//! lint, so the report is an error here.
#![deny(unused_variables)]

#[whichever::dispatchable]
pub trait Probe {
    fn base(&self) -> u64;
    fn m(&self, x: u64) -> u64;
}

pub struct P0;

impl Probe for P0 {
    fn base(&self) -> u64 {
        0
    }
    fn m(&self, x: u64) -> u64 {
        x
    }
}

pub struct P1;

impl Probe for P1 {
    fn base(&self) -> u64 {
        1
    }
    fn m(&self, x: u64) -> u64 {
        x + 1
    }
}

#[whichever::dispatch]
pub enum E {
    #[cfg(any())]
    A(P0),
    #[cfg(any())]
    B(P1),
}

#[whichever::dispatch]
impl Probe for E {
    #[dispatch(A)]
    fn m(&self, x: u64) -> u64 {
        7
    }
}

fn main() {}
