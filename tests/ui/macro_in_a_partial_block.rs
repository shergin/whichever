//! A macro invocation in a partial impl block is not read, so an item it
//! writes would be forwarded as well and defined twice.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
    fn sides(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn sides(&self) -> u32 {
        4
    }
}

#[allow(unused_macros)]
macro_rules! sides {
    () => {
        fn sides(&self) -> u32 {
            4
        }
    };
}

#[whichever::dispatch]
enum Any {
    Square(Square),
}

#[whichever::dispatch]
impl Shape for Any {
    sides!();
}

fn main() {}
