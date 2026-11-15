//! A route ending in a method call whose result does not fit the receiver,
//! `.copied()` under `&self`, is a forwarding no hand can write either; it
//! is rustc's to reject, and its error stands on the enum's name, where
//! the arm is, not on the trait's attribute.
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
