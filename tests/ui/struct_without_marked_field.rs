#[whichever::dispatchable]
trait Area {
    fn area(&self) -> u32;
}

struct Square(u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Area)]
struct Labelled {
    shape: Square,
    label: &'static str,
}

fn main() {}
