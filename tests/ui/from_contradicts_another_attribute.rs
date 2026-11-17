//! The `From` impls are the enum's and written once, so `from` takes one
//! value across the enum's attributes; the later of two that disagree is
//! reported, in either order.
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

#[whichever::dispatch(Low, from = true)]
#[whichever::dispatch(High, from = false)]
enum OnThenOff {
    One(One),
}

#[whichever::dispatch(Low, from = false)]
#[whichever::dispatch(High, from = true)]
enum OffThenOn {
    One(One),
}

fn main() {}
