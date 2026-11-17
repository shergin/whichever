//! A method without a receiver that takes a `Self` argument has no `match`
//! to bind it in, so a required one is reported on a struct.
#[whichever::dispatchable]
trait Merge {
    fn merged(left: &Self, right: &Self) -> u32;
    fn size(&self) -> u32;
}

struct Square(u32);

impl Merge for Square {
    fn merged(left: &Self, right: &Self) -> u32 {
        left.0 + right.0
    }
    fn size(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Merge)]
struct Wrapper(Square);

fn main() {}
