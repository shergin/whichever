//! A method taking `self` by value cannot be forwarded through a pointer
//! that is not a `Box`: nothing can be moved out of it.
use std::rc::Rc;

#[whichever::dispatchable]
trait Consume {
    fn consume(self) -> u32;
}

struct Inner(u32);

impl Consume for Inner {
    fn consume(self) -> u32 {
        self.0
    }
}

struct Account {
    base: Inner,
}

#[whichever::dispatch(Consume)]
enum Any<'a> {
    #[dispatch(via = *.base)]
    Shared(Rc<Account>),
    #[dispatch(deref)]
    Borrowed(&'a Inner),
    #[dispatch(deref)]
    Boxed(Box<Inner>),
}

fn main() {}
