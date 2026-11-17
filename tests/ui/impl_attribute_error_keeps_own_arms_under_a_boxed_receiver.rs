//! A rejected `#[dispatch]` on an inherent block keeps the block as
//! written, and a method under `#[dispatch(*)]` whose body holds arms of
//! its own is kept closed, matching the enum moved out of its box as the
//! forwarding would: the one error stands alone.
pub struct Circle(pub u32);

impl Circle {
    pub fn take(self: Box<Self>) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
pub enum Shape {
    Circle(Circle),
    Empty,
}

#[whichever::dispatch(Shape)]
impl Shape {
    #[dispatch(*)]
    pub fn take(self: Box<Self>) -> u32 {
        match self {
            Self::Empty => 0,
        }
    }
}

fn main() {}
