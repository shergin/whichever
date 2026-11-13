//! `#[dispatch(default)]` under `#[cfg]` leaves a method to the trait's
//! default where the `cfg` holds, so the method has to have one there;
//! `#[cfg(all())]` stands for a `cfg` that holds in this build.
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
    #[cfg(all())]
    #[dispatch(default)]
    fn price(&self) -> u32;
}

fn main() {}
