//! A method's mark on a struct that names nothing, or names a field with a
//! body that would never run, speaks of the struct's field.
pub struct Square(pub u32);

impl Square {
    pub fn side(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
pub struct Labelled {
    #[dispatch]
    square: Square,
    pub label: &'static str,
}

#[whichever::dispatch]
impl Labelled {
    #[dispatch]
    pub fn bare(&self) -> u32;

    #[dispatch()]
    pub fn empty(&self) -> u32;

    #[dispatch(square)]
    pub fn side(&self) -> u32 {
        0
    }
}

fn main() {}
