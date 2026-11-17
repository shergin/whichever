//! A trait's route on a struct keyed by a field other than the one the
//! trait is forwarded to is told where the trait goes.
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

struct Holder {
    square: Square,
}

#[whichever::dispatch(Shape, via(holder = .square))]
struct Labelled {
    #[dispatch]
    square: Square,
    holder: Holder,
}

fn main() {}
