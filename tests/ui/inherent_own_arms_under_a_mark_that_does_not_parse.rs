pub struct Circle;

impl Circle {
    pub fn area(&self) -> u32 {
        3
    }
}

#[whichever::dispatch]
pub enum Shape {
    Circle(Circle),
    Empty,
}

#[whichever::dispatch]
impl Shape {
    #[dispatch(*, call = Circle::area)]
    pub fn area(&self) -> u32 {
        match self {
            Self::Empty => 0,
        }
    }
}

fn main() {}
