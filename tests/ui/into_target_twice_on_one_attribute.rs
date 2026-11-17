//! An `into` target named twice on one attribute is reported on the
//! second, as across two.
#[whichever::dispatchable]
trait Low {
    fn low(&self) -> u8;
}

struct One(u8);

impl Low for One {
    fn low(&self) -> u8 {
        self.0
    }
}

impl From<One> for u32 {
    fn from(one: One) -> Self {
        u32::from(one.0)
    }
}

#[whichever::dispatch(Low, into = u32, into = u32)]
enum Kind {
    One(One),
}

fn main() {}
