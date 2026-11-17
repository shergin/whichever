//! A mark under `cfg_attr` on a variant or on a variant's field is read in
//! no build; it is one sentence naming the way around, and the variant is
//! read as if the mark were plain.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape)]
pub enum Any {
    #[cfg_attr(all(), dispatch(deref))]
    Boxed(Box<Square>),
    Named {
        #[cfg_attr(all(), dispatch)]
        shape: Square,
        id: u8,
    },
}

fn main() {}
