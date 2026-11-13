use std::pin::Pin;

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn grow(&mut self, by: u32);
    fn into_area(self) -> u32;
    fn poll_area(self: Pin<&mut Self>) -> u32;
}

pub struct Circle(pub u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
    fn grow(&mut self, by: u32) {
        self.0 += by;
    }
    fn into_area(self) -> u32 {
        self.area()
    }
    fn poll_area(self: Pin<&mut Self>) -> u32 {
        self.area()
    }
}

// Every variant is under `#[cfg]`, so the enum is empty in some build, and
// a reference receiver's `match` looks at the enum itself.
#[whichever::dispatch(Shape)]
pub enum AnyShape {
    #[cfg(all())]
    Circle(Circle),
    #[cfg(any())]
    Mock(Circle),
}

fn main() {}
