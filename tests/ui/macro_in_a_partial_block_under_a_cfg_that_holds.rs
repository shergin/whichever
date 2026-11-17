//! A macro invocation in a partial impl block under `#[cfg]` is reported
//! where the `cfg` holds, as one without a `cfg` is.
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
    #[cfg(all())]
    sides!();
}

fn main() {}
