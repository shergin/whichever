//! A partial block's mark under the trait method's own `#[cfg]` leaves the
//! plain forwarding no build that has the method, so a variant no method
//! can forward to is not reported: the behaviour tests are compiled with
//! `test` set, which is the build where the trait has the methods.

#[whichever::dispatchable]
pub trait Speak {
    fn speak(&self) -> u8;
    #[cfg(test)]
    fn extra(&self) -> u8 {
        0
    }
    #[cfg(test)]
    fn named(&self) -> u8 {
        0
    }
}

pub struct A;

impl Speak for A {
    fn speak(&self) -> u8 {
        1
    }
    #[cfg(test)]
    fn named(&self) -> u8 {
        5
    }
}

#[whichever::dispatch]
pub enum Voice {
    A(A),
    Other(u8, u8),
}

#[whichever::dispatch]
impl Speak for Voice {
    #[dispatch(A)]
    fn speak(&self) -> u8 {
        9
    }

    #[cfg(test)]
    #[dispatch(default)]
    fn extra(&self) -> u8;

    #[cfg(test)]
    #[dispatch(A)]
    fn named(&self) -> u8 {
        8
    }
}

#[test]
fn a_default_mark_takes_the_trait_default() {
    assert_eq!(Voice::A(A).extra(), 0);
    assert_eq!(Voice::Other(1, 2).extra(), 0);
}

#[test]
fn a_named_mark_forwards_its_variants_and_answers_the_rest() {
    assert_eq!(Voice::A(A).named(), 5);
    assert_eq!(Voice::Other(1, 2).named(), 8);
    assert_eq!(Voice::A(A).speak(), 1);
    assert_eq!(Voice::Other(1, 2).speak(), 9);
}
