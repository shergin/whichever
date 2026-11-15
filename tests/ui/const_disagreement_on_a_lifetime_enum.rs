//! An enum with lifetime parameters alone has its associated consts checked
//! as any other: no lifetime changes a const's value, so the check compares
//! the payloads with each lifetime `'static`, and a disagreement is reported
//! on the variant.
#[whichever::dispatchable]
trait Leveled {
    const LEVEL: u32;
    fn level(&self) -> u32;
}

struct Low;
struct High;

impl Leveled for Low {
    const LEVEL: u32 = 7;
    fn level(&self) -> u32 {
        Self::LEVEL
    }
}

impl Leveled for High {
    const LEVEL: u32 = 8;
    fn level(&self) -> u32 {
        Self::LEVEL
    }
}

#[whichever::dispatch(Leveled)]
enum Any<'a> {
    Low(Low),
    #[dispatch(deref)]
    Borrowed(&'a High),
}

fn main() {}
