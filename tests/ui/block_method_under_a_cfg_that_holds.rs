//! A method of a `#[dispatch]` impl block that cannot be forwarded is
//! reported under its own `#[cfg]`: where the `cfg` holds, its sentence
//! stands as it would without one.
pub struct Circle(pub u32);

impl Circle {
    pub fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
}

#[whichever::dispatch]
pub enum Shape {
    Circle(Circle),
}

#[whichever::dispatch]
impl Shape {
    #[cfg(all())]
    #[dispatch(*)]
    pub fn make(side: u32) -> Self;

    #[cfg(all())]
    #[dispatch(*, Circle)]
    pub fn doubled(&self) -> u32;

    #[cfg(all())]
    pub fn unmarked(&self) -> u32;

    #[dispatch(*)]
    pub fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Square {
    pub fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch]
pub struct Labelled {
    #[dispatch]
    shape: Square,
    pub label: &'static str,
}

#[whichever::dispatch]
impl Labelled {
    #[cfg(all())]
    #[dispatch(*, shape)]
    pub fn doubled(&self) -> u32;

    #[dispatch(*)]
    pub fn area(&self) -> u32;
}

fn main() {}
