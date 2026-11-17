//! A variant and its marked field each give a route for a mutable borrow;
//! one is the route, so the second is reported.
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

#[whichever::dispatch(Grow)]
pub enum Any {
    #[dispatch(via_mut = .base_mut())]
    Tagged(#[dispatch(via = .base(), via_mut = .base_mut())] Account, u32),
}

fn main() {}
