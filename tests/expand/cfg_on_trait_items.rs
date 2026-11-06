//! A `#[cfg]` on a trait item is decided where the trait is: the generated
//! item exists exactly when the trait's does, and carries no `cfg`.
#[whichever::dispatchable]
pub trait Counted {
    fn plain(&self) -> u32;
    #[cfg(all())]
    fn present(&self) -> u32;
    #[cfg(any())]
    fn absent(&self) -> u32;
}

pub struct One;

impl Counted for One {
    fn plain(&self) -> u32 {
        1
    }
    fn present(&self) -> u32 {
        2
    }
}

#[whichever::dispatch(Counted)]
pub enum Any {
    One(One),
}

fn main() {}
