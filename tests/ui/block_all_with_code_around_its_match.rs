#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u32;
}

pub struct Circle;

impl Area for Circle {
    fn area(&self) -> u32 {
        3
    }
}

#[whichever::dispatch]
pub enum Shape {
    Circle(Circle),
    Empty,
}

#[whichever::dispatch]
impl Area for Shape {
    #[dispatch(*)]
    fn area(&self) -> u32 {
        if false {
            return 0;
        }
        match self {
            Self::Empty => 0,
        }
    }
}

fn main() {}
