//! The trait's parameter names are kept in the generated impl, whatever they
//! are: the engine's own bindings are hygienic and cannot collide with them.

#[whichever::dispatchable]
pub trait Mixer {
    #[allow(clippy::too_many_arguments)]
    fn mix(
        &self,
        __inner: u32,
        __same0: u32,
        __value: u32,
        __other: u32,
        __argument0: u32,
        inner: u32,
    ) -> u32;
    fn pair(&self, (a, b): (u32, u32), _: u32, mut rest: u32) -> u32 {
        rest += a + b;
        rest
    }
    #[dispatch(mismatch = __inner + inner)]
    fn beside(&self, __inner: u32, other: &Self, inner: u32) -> u32;
}

pub struct Sum;
pub struct Zero;

impl Mixer for Sum {
    fn mix(
        &self,
        __inner: u32,
        __same0: u32,
        __value: u32,
        __other: u32,
        __argument0: u32,
        inner: u32,
    ) -> u32 {
        __inner + __same0 + __value + __other + __argument0 + inner
    }
    fn beside(&self, __inner: u32, _other: &Self, inner: u32) -> u32 {
        __inner * inner
    }
}

impl Mixer for Zero {
    fn mix(&self, _: u32, _: u32, _: u32, _: u32, _: u32, _: u32) -> u32 {
        0
    }
    fn beside(&self, _: u32, _: &Self, _: u32) -> u32 {
        0
    }
}

#[whichever::dispatch(Mixer)]
pub enum Any {
    Sum(Sum),
    Zero(Zero),
}

#[test]
fn every_name_forwards() {
    let sum = Any::Sum(Sum);
    assert_eq!(sum.mix(1, 2, 3, 4, 5, 6), 21);
    assert_eq!(sum.pair((1, 2), 9, 3), 6);
    assert_eq!(sum.beside(2, &Any::Sum(Sum), 3), 6);
    assert_eq!(sum.beside(2, &Any::Zero(Zero), 3), 5);
}
