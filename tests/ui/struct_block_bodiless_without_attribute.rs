//! A bodiless method of a partial block over a struct, without a mark,
//! says how to forward it to the struct's field.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch]
struct Wrapper(Square);

#[whichever::dispatch]
impl Shape for Wrapper {
    fn area(&self) -> u32;
}

fn main() {}
