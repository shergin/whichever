//! Pins how a sentence spells a payload: a lifetime, `mut`, `dyn` and a
//! comma keep their space.
use std::rc::Rc;

pub struct Name(pub String);

pub trait Named {}

pub struct Pair<'a, T>(pub &'a T);

#[whichever::dispatch]
pub enum Held<'a> {
    #[dispatch(deref)]
    Borrowed(&'a Name),
    #[dispatch(deref)]
    Exclusive(&'a mut Name),
    #[dispatch(deref)]
    Object(Rc<dyn Named + 'a>),
    #[dispatch(deref)]
    Paired(Rc<Pair<'a, Name>>),
}

#[whichever::dispatch]
impl<'a> Held<'a> {
    #[dispatch(Borrowed, value, with = Some)]
    pub fn borrowed(self) -> Option<Name> {
        None
    }

    #[dispatch(Exclusive, value, with = Some)]
    pub fn exclusive(self) -> Option<Name> {
        None
    }

    #[dispatch(Object, value, with = Some)]
    pub fn object(self) -> Option<Name> {
        None
    }

    #[dispatch(Paired, value, with = Some)]
    pub fn paired(self) -> Option<Name> {
        None
    }
}

fn main() {}
