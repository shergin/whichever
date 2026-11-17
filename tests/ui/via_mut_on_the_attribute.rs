//! A route for a mutable borrow after a trait is keyed by what it goes
//! through, `via_mut(Variant = route)`, as `via(..)` is; and on an attribute
//! that names no trait it is reported, and read beside the one trait the
//! enum dispatches, so the sentence stands alone.
#[whichever::dispatchable]
pub trait Grow {
    fn size(&self) -> u32;
    fn grow(&mut self);
}

pub struct Square(pub u32);

impl Grow for Square {
    fn size(&self) -> u32 {
        self.0
    }
    fn grow(&mut self) {
        self.0 += 1;
    }
}

pub struct Account {
    base: Square,
}

impl Account {
    pub fn base(&self) -> &Square {
        &self.base
    }
    pub fn base_mut(&mut self) -> &mut Square {
        &mut self.base
    }
}

#[whichever::dispatch(Grow, via_mut = .base_mut())]
pub enum Unkeyed {
    #[dispatch(via = .base())]
    Module(Account),
}

#[whichever::dispatch(Grow)]
#[whichever::dispatch(via_mut(Module = .base_mut()))]
pub enum Stray {
    #[dispatch(via = .base())]
    Module(Account),
}

fn main() {}
