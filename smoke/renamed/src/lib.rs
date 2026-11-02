//! The facade crate under another dependency name: the enum side has to
//! find it through `proc-macro-crate`.

#[wh::dispatchable]
pub trait Price {
    fn price(&self) -> u64;
}

pub struct Fixed(pub u64);

impl Price for Fixed {
    fn price(&self) -> u64 {
        self.0
    }
}

#[wh::dispatch(Price)]
pub enum Quote {
    Fixed(Fixed),
}

pub fn probe(quote: &Quote) -> u64 {
    quote.price()
}
