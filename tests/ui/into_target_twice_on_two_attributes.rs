//! `From<Enum> for Target` is the enum's and written once, so an `into`
//! target named on two attributes is reported on the second.
#[whichever::dispatchable]
trait Low {
    fn low(&self) -> u8;
}

#[whichever::dispatchable]
trait High {
    fn high(&self) -> u8;
}

struct One(u8);

impl Low for One {
    fn low(&self) -> u8 {
        self.0
    }
}

impl High for One {
    fn high(&self) -> u8 {
        self.0
    }
}

impl From<One> for u32 {
    fn from(one: One) -> Self {
        u32::from(one.0)
    }
}

#[whichever::dispatch(Low, into = u32)]
#[whichever::dispatch(High, into = u32)]
enum Kind {
    One(One),
}

fn main() {}
