//! A struct holds every field in every value, so a method is forwarded to
//! one; naming two would leave the second an arm no value reaches.
pub struct Square(pub u32);
pub struct Circle(pub u32);

impl Square {
    pub fn side(&self) -> u32 {
        self.0
    }
}

impl Circle {
    pub fn side(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
pub struct Holder {
    pub square: Square,
    pub circle: Circle,
}

#[whichever::dispatch]
impl Holder {
    #[dispatch(square, circle)]
    pub fn side(&self) -> u32;
}

fn main() {}
