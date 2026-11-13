//! A place under a lift is borrowed as the lift is handed: the return type's
//! first argument is a reference, so the arm is `Some(&inner.id)`, and a
//! `Copy` field under `Option<u32>` stays by value.
pub struct Card {
    pub id: String,
    pub n: u32,
}

pub struct Square(pub u32);

#[whichever::dispatch(from = false)]
pub enum Held {
    Plain(Card),
    Other(Square),
}

#[whichever::dispatch]
impl Held {
    #[dispatch(Plain, field = id, with = Some)]
    pub fn id_if_plain(&self) -> Option<&String> {
        None
    }

    #[dispatch(Plain, field = id, with = Some)]
    pub fn id_if_plain_mut(&mut self) -> Option<&mut String> {
        None
    }

    #[dispatch(Plain, field = n, with = Some)]
    pub fn n_if_plain(&self) -> Option<u32> {
        None
    }
}

fn main() {}
