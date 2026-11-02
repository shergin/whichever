#[whichever::dispatchable]
trait Grow {
    fn grown(&self) -> Self
    where
        Self: Sized;
}

struct Square(u32);

impl Grow for Square {
    fn grown(&self) -> Self {
        Square(self.0 + 1)
    }
}

#[whichever::dispatch(Grow)]
struct Labelled {
    #[dispatch]
    shape: Square,
    label: &'static str,
}

fn main() {}
