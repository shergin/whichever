//! With every variant under `#[cfg]`, a method under a reference receiver
//! is matched by place and takes no arms of its own, so a variant without
//! a payload under `#[dispatch(*)]` names the named list with a body as
//! the way around, not an arm.
pub struct IntBounds(pub u32);

impl IntBounds {
    pub fn len(&self) -> usize {
        1
    }
}

#[whichever::dispatch]
pub enum Bounds {
    #[cfg(all())]
    Int(IntBounds),
    #[cfg(all())]
    NoOrder,
}

#[whichever::dispatch]
impl Bounds {
    #[dispatch(*)]
    pub fn len(&self) -> usize;
}

fn main() {}
