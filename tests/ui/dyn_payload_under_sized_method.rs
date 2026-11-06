//! A trait object payload cannot take a method that requires `Self: Sized`;
//! the way out is to leave that method to the trait's default.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
    fn doubled(&self) -> u32
    where
        Self: Sized,
    {
        self.area() * 2
    }
}

struct Square(u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape)]
enum Any {
    Plain(Square),
    #[dispatch(deref)]
    Boxed(Box<dyn Shape>),
}

fn main() {}
