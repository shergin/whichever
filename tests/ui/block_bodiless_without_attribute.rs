//! A method without a body in a partial block is the engine's to write,
//! and says how.
#[whichever::dispatchable]
trait Priced {
    fn price(&self) -> u32;
}

struct Book(u32);

impl Priced for Book {
    fn price(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
enum Item {
    Book(Book),
}

#[whichever::dispatch]
impl Priced for Item {
    fn price(&self) -> u32;
}

fn main() {}
