//! A `Self` cannot be wrapped back through a route: the other fields are
//! gone.
#[whichever::dispatchable]
trait Grown {
    fn grown(&self) -> Self;
}

struct Square(u32);

impl Grown for Square {
    fn grown(&self) -> Self {
        Square(self.0 + 1)
    }
}

struct Account {
    base: Square,
}

#[whichever::dispatch(Grown)]
enum Any {
    #[dispatch(via = .base)]
    Module(Account),
}

fn main() {}
