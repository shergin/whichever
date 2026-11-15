//! A method returning `Self` cannot be forwarded to the marked field of a
//! variant with several: the rebuilt variant would lack the other fields.
//! The sentence names the variant, not the enum.
#[whichever::dispatchable]
trait Grow {
    fn grown(&self) -> Self;
}

struct Square(u32);

impl Grow for Square {
    fn grown(&self) -> Self {
        Square(self.0 + 1)
    }
}

#[whichever::dispatch(Grow)]
enum Shape {
    Tagged(#[dispatch] Square, u32),
    Plain(Square),
}

fn main() {}
