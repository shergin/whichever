//! A payload that is a type parameter of the enum gets the bound a hand
//! writes on the impl, behind a pointer too.
#[whichever::dispatchable]
pub trait Weigh {
    fn weight(&self) -> u32;
}

#[whichever::dispatch(Weigh, from = false)]
pub enum Lazy<Raw, Parsed> {
    Raw(Raw),
    #[dispatch(deref)]
    Parsed(Box<Parsed>),
}

#[whichever::dispatch(Weigh)]
pub struct Wrapper<T>(T);

fn main() {}
