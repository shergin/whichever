pub struct Circle(pub u32);
pub struct Square(pub u32);

impl Circle {
    pub fn radius(&self) -> u32 {
        self.0
    }
    pub fn describe(&self) -> String {
        format!("circle {}", self.0)
    }
}

impl Square {
    pub fn describe(&self) -> String {
        format!("square {}", self.0)
    }
}

#[whichever::dispatch(try_from = true)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
    #[dispatch(deref)]
    Boxed(Box<Square>),
}

#[whichever::dispatch]
impl AnyShape {
    /// Only circles have a radius.
    #[dispatch(Circle, with = Some)]
    pub fn radius(&self) -> Option<u32> {
        None
    }

    #[dispatch(*)]
    pub fn describe(&self) -> String;

    #[dispatch(*, field = 0)]
    pub fn size(&self) -> &u32;
}

fn main() {}
