//! A route ending in a method call yields a value, which is a `&mut` only
//! where its method lends one; without a route named for a mutable borrow,
//! `&mut self` over a getter that lends shared is reported, naming
//! `via_mut`.
#[whichever::dispatchable]
trait Grow {
    fn grow(&mut self);
}

struct Square(u32);

impl Grow for Square {
    fn grow(&mut self) {
        self.0 += 1;
    }
}

struct Account {
    base: Square,
}

impl Account {
    fn base(&self) -> &Square {
        &self.base
    }
}

#[whichever::dispatch(Grow)]
enum Any {
    #[dispatch(via = .base())]
    Module(Account),
}

fn main() {}
