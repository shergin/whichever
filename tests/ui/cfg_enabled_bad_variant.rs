//! A variant that is not `Name(Payload)` is an error exactly when it is
//! compiled in; `#[cfg(all())]` stands for a test-only variant in a test
//! build.
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
enum Shape {
    Square(Square),
    #[cfg(all())]
    Mock,
}

fn main() {}
