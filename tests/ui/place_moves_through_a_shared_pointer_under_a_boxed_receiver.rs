//! Under `self: Box<Self>` the enum is matched by value, so a place taken by
//! value cannot be read through a pointer that is not a `Box`: nothing can
//! be moved out of it.
use std::rc::Rc;

pub struct Name(pub String);

#[whichever::dispatch]
pub enum E {
    #[dispatch(deref)]
    Shared(Rc<Name>),
    #[dispatch(deref)]
    Boxed(Box<Name>),
    Plain(Name),
}

#[whichever::dispatch]
impl E {
    #[dispatch(*, value)]
    pub fn into_name(self: Box<Self>) -> Name;

    #[dispatch(*, field = 0)]
    pub fn into_text(self: Box<Self>) -> String;
}

fn main() {}
