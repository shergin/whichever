//! A place under a lift is borrowed as the lift is handed: the return type's
//! first argument is a reference, so the arm is `Some(&inner.id)`, and a
//! `Copy` field under `Option<u32>` stays by value.
pub struct Card {
    pub id: String,
    pub n: u32,
}
pub struct Square(pub u32);
pub enum Held {
    Plain(Card),
    Other(Square),
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Held_5592bf1fdad0e190_11_1 as Held;
impl Held {
    pub fn id_if_plain(&self) -> Option<&String> {
        match self {
            Self::Plain(__inner) => Some(&__inner.id),
            _ => None,
        }
    }
    pub fn id_if_plain_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Plain(__inner) => Some(&mut __inner.id),
            _ => None,
        }
    }
    pub fn n_if_plain(&self) -> Option<u32> {
        match self {
            Self::Plain(__inner) => Some(__inner.n),
            _ => None,
        }
    }
}
fn main() {}
