//! A tuple under an `Option` would need a closure to rewrap, which the
//! attribute does not write.
#[whichever::dispatchable]
trait Split {
    fn split(&self) -> Option<(Self, u32)>
    where
        Self: Sized;
}

struct Even(u32);

impl Split for Even {
    fn split(&self) -> Option<(Self, u32)> {
        Some((Even(self.0 / 2), 2))
    }
}

#[whichever::dispatch(Split)]
enum Number {
    Even(Even),
}

fn main() {}
