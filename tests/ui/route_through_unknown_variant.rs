//! A trait's route names a variant the enum has.
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

#[whichever::dispatch(Area, via(Circle = .0))]
enum Any {
    Plain(Square),
}

fn main() {}
