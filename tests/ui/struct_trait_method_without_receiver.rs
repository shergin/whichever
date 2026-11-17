//! A trait method without a receiver is the struct's field's type's, where
//! that type can be named; through a route it cannot, so a required one is
//! reported, and so is an inherent one, each with its own way around.
#[whichever::dispatchable]
trait Shape {
    fn kind() -> &'static str;
    fn area(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn kind() -> &'static str {
        "square"
    }
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

impl Square {
    fn unit() -> Self {
        Square(1)
    }
}

struct Holder {
    square: Square,
}

#[whichever::dispatch(Shape)]
struct Wrapper {
    #[dispatch(via = .square)]
    holder: Holder,
}

#[whichever::dispatch]
impl Wrapper {
    #[dispatch(*, call = unit)]
    fn unit_square() -> Square;
}

fn main() {}
