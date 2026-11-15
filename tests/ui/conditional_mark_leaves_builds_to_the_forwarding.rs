//! A mark under a `cfg` the trait method does not carry leaves the builds
//! without it to the plain forwarding, which cannot reach a variant without
//! a payload: the variant is reported there. `#[cfg(any())]` stands for a
//! `cfg` that does not hold in this build, such as a feature left off.
#[whichever::dispatchable]
pub trait Speak {
    fn speak(&self) -> u8;
}

pub struct A;

impl Speak for A {
    fn speak(&self) -> u8 {
        1
    }
}

#[whichever::dispatch]
pub enum Voice {
    A(A),
    Other(u8, u8),
}

#[whichever::dispatch]
impl Speak for Voice {
    #[cfg(any())]
    #[dispatch(A)]
    fn speak(&self) -> u8 {
        9
    }
}

fn main() {}
