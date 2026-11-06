//! An enum with one variant that is always compiled in has no second
//! variant to meet, so a method taking `Self` arguments needs no `mismatch`
//! and gets no fallback arm, as a hand writes none.

#[whichever::dispatchable]
pub trait Same {
    fn same(&self, other: &Self) -> bool;
}

pub struct A(pub u8);

impl Same for A {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[whichever::dispatch(Same)]
pub enum Only {
    A(A),
}

#[test]
fn the_one_arm_is_exhaustive() {
    assert!(Only::A(A(1)).same(&Only::A(A(1))));
    assert!(!Only::A(A(1)).same(&Only::A(A(2))));
}
