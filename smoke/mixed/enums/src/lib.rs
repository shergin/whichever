//! An enum dispatching the other crate's trait with an older `whichever`.
use mixed_traits::Device;

pub struct A;

impl Device for A {
    fn id(&self) -> u32 {
        1
    }
}

#[whichever::dispatch(Device)]
pub enum Any {
    A(A),
}
