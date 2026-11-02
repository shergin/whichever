#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Area)]
pub enum Shape<'a> {
    #[dispatch(deref)]
    Boxed(Box<Square>),
    #[dispatch(deref)]
    Borrowed(&'a Square),
    Plain(Square),
}

fn main() {}
