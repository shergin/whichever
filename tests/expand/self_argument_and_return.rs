#[whichever::dispatchable]
pub trait Shape {
    #[dispatch(mismatch = false)]
    fn congruent(&self, other: &Self) -> bool;
    fn scaled(&self, factor: u32) -> Self;
}

pub struct Circle(u32);
pub struct Square(u32);

impl Shape for Circle {
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn scaled(&self, factor: u32) -> Self {
        Circle(self.0 * factor)
    }
}

impl Shape for Square {
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn scaled(&self, factor: u32) -> Self {
        Square(self.0 * factor)
    }
}

#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}

fn main() {}
