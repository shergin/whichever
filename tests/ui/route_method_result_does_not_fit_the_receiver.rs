//! A route ending in a method call whose result does not fit the receiver,
//! `.copied()` under `&self`, hands a value where a reference is asked
//! for. A hand would borrow the temporary, `&inner.copied()`, which the
//! attribute does not write; the arm is rustc's to reject, and its error
//! stands on the method's name in the route, where the result comes from.
#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u64;
}

#[derive(Clone, Copy)]
pub struct Square(pub u64);

impl Area for Square {
    fn area(&self) -> u64 {
        self.0 * self.0
    }
}

pub struct Slot(pub Square);

impl Slot {
    pub fn copied(&self) -> Square {
        self.0
    }
}

#[whichever::dispatch(Area)]
pub enum Shape {
    #[dispatch(via = .copied())]
    Held(Slot),
}

fn main() {}
