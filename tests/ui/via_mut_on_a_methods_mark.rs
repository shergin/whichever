//! A method's own route is followed whatever the method borrows, so its mark
//! takes `via` alone, and `via_mut` there is reported.
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

#[whichever::dispatch]
pub enum Any {
    Module(Account),
}

#[whichever::dispatch]
impl Any {
    #[dispatch(*, via_mut = .base_mut(), call = grow)]
    pub fn grow(&mut self);
}

fn main() {}
