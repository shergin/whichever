//! With every variant under `#[cfg]`, a method under a reference receiver
//! is matched as `match *self` with `ref` bindings, which arms written for
//! `match self` are not.
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
    pub fn len(&self) -> usize {
        match self {
            Self::NoOrder => 0,
        }
    }
}

fn main() {}
