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
    #[dispatch(*)]
    pub fn area(&self) -> u32 {
        if false {
            return 0;
        }
        match self {
            Self::Empty => 0,
        }
    }
}

fn main() {}
