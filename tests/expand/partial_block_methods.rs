//! A method of a partial block names the variants it forwards to, with its
//! body for the rest; `#[dispatch(default)]` leaves one to the trait.
#[whichever::dispatchable]
pub trait Priced {
    fn price(&self) -> u32;
    fn label(&self) -> &'static str {
        "priced"
    }
}

pub struct Book(pub u32);

impl Priced for Book {
    fn price(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
pub enum Item {
    Book(Book),
    Gift,
}

#[whichever::dispatch]
impl Priced for Item {
    #[dispatch(Book)]
    fn price(&self) -> u32 {
        0
    }
    #[dispatch(default)]
    fn label(&self) -> &'static str;
}

fn main() {}
