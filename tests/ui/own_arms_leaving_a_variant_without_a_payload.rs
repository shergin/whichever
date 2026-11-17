//! Under `#[dispatch(*)]` a method's body holds the arms the enum answers
//! itself, and every other variant is forwarded: a variant that holds no
//! payload and has no arm is reported, and the sentence names the arm to
//! write for it.
pub struct IntBounds(pub u32);

impl IntBounds {
    pub fn len(&self) -> usize {
        1
    }
}

#[whichever::dispatch]
pub enum Bounds {
    Int(IntBounds),
    Unknown(u32),
    NoOrder,
}

#[whichever::dispatch]
impl Bounds {
    #[dispatch(*)]
    pub fn len(&self) -> usize {
        match self {
            Self::Unknown(..) => 0,
        }
    }
}

fn main() {}
