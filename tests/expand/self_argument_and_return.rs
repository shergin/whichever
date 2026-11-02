#[whichever::dispatchable]
pub trait Swap {
    #[dispatch(mismatch = false)]
    fn pricing_eq(&self, other: &Self) -> bool;
    fn with_rate(&self, rate: u64) -> Self;
}

pub struct Pool(u64);
pub struct Book(u64);

impl Swap for Pool {
    fn pricing_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn with_rate(&self, rate: u64) -> Self {
        Pool(rate)
    }
}

impl Swap for Book {
    fn pricing_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn with_rate(&self, rate: u64) -> Self {
        Book(rate)
    }
}

#[whichever::dispatch(Swap)]
pub enum SwapKind {
    Pool(Pool),
    Book(Book),
}

fn main() {}
