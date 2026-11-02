#[whichever::dispatchable]
trait Scale {
    fn scaled(&self, factor: u32) -> Self;
}

struct Square(u32);

impl Scale for Square {
    fn scaled(&self, factor: u32) -> Self {
        Square(self.0 * factor)
    }
}

#[whichever::dispatch(Scale)]
enum Shape {
    #[dispatch(deref)]
    Boxed(Box<Square>),
}

fn main() {}
