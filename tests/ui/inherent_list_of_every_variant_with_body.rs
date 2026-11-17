//! A named list that names every variant leaves its body for the rest no
//! variant to answer for.
pub struct Circle;

impl Circle {
    pub fn radius(&self) -> u32 {
        1
    }
}

pub struct Square;

impl Square {
    pub fn radius(&self) -> u32 {
        2
    }
}

#[whichever::dispatch]
pub enum Shape {
    Circle(Circle),
    Square(Square),
}

#[whichever::dispatch]
impl Shape {
    #[dispatch(Circle, Square)]
    pub fn radius(&self) -> u32 {
        0
    }
}

fn main() {}
