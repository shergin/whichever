//! `#[dispatch(default)]` leaves a method to the trait's default, which the
//! method has to have.
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
    #[dispatch(default)]
    fn price(&self) -> u32;
}

fn main() {}
