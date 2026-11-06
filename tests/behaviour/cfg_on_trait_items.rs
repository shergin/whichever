//! A `#[cfg]` on a trait item is decided where the trait is: the carrier is
//! written once per combination of the predicates, and the generated item
//! exists exactly when the trait's does.

#[whichever::dispatchable]
pub trait Counted {
    #[cfg(test)]
    const PRESENT: u32;
    #[cfg(not(test))]
    const ABSENT: u32;
    fn plain(&self) -> u32;
    #[cfg(test)]
    fn present(&self) -> u32;
    #[cfg(not(test))]
    fn absent(&self) -> u32;
}

pub struct One;

impl Counted for One {
    const PRESENT: u32 = 7;
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

#[test]
fn the_items_that_exist_are_forwarded() {
    let any = Any::One(One);
    assert_eq!((any.plain(), any.present(), Any::PRESENT), (1, 2, 7));
}
