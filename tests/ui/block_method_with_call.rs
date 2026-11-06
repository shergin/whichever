//! A trait method is called by its own name on every variant it names;
//! `call`, `with`, `field` and `value` are for inherent methods.
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
    Gift,
}

#[whichever::dispatch]
impl Priced for Item {
    #[dispatch(Book, call = cost)]
    fn price(&self) -> u32 {
        0
    }
}

fn main() {}
