//! One field of a variant with several is the payload; two marks are one
//! too many.
#[whichever::dispatchable]
trait Area {
    fn area(&self) -> u32;
}

struct Square(u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Area)]
enum Any {
    Both(#[dispatch] Square, #[dispatch] Square),
}

fn main() {}
