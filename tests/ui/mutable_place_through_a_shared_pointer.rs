//! An inherent method whose place is returned as `&mut`, through a `deref`
//! variant over an `Rc`, would borrow mutably what the pointer lends shared
//! only.
use std::rc::Rc;

pub struct Name(pub String);

#[whichever::dispatch]
pub enum Named {
    #[dispatch(deref)]
    Shared(Rc<Name>),
}

#[whichever::dispatch]
impl Named {
    #[dispatch(*, value)]
    pub fn name(&mut self) -> &mut Name;
}

fn main() {}
