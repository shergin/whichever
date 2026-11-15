//! Each variant whose associated const differs from the first variant's is
//! reported on its own, not the first of them alone: every check is an
//! item evaluated on its own.
#[whichever::dispatchable]
trait Leveled {
    const LEVEL: u32;
    fn level(&self) -> u32;
}

struct Low;
struct Middle;
struct High;

impl Leveled for Low {
    const LEVEL: u32 = 7;
    fn level(&self) -> u32 {
        Self::LEVEL
    }
}

impl Leveled for Middle {
    const LEVEL: u32 = 8;
    fn level(&self) -> u32 {
        Self::LEVEL
    }
}

impl Leveled for High {
    const LEVEL: u32 = 9;
    fn level(&self) -> u32 {
        Self::LEVEL
    }
}

#[whichever::dispatch(Leveled)]
enum Any {
    Low(Low),
    Middle(Middle),
    High(High),
}

fn main() {}
