//! A method without a receiver returning `Self` builds the struct around
//! what the field's type returns, which leaves the other fields nothing.
#[whichever::dispatchable]
trait Unit {
    fn unit() -> Self;
    fn size(&self) -> u32;
}

struct Square(u32);

impl Unit for Square {
    fn unit() -> Self {
        Square(1)
    }
    fn size(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Unit)]
struct Labelled {
    #[dispatch]
    square: Square,
    label: &'static str,
}

fn main() {}
