//! An arm reaches its target through a route: a field of the payload, the
//! marked field of a variant with several, or what a method returns; the
//! call is then qualified by the trait alone, since the target's type is
//! not the payload's.
#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u32;
    fn grow(&mut self, by: u32);
}

pub struct Square(pub u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn grow(&mut self, by: u32) {
        self.0 += by;
    }
}

pub struct Account {
    pub base: Square,
    pub id: u32,
}

#[whichever::dispatch(Area, via(Pair = .1))]
pub enum Any {
    #[dispatch(via = .base)]
    Module(Account),
    Pair((u32, Square)),
    Tagged(#[dispatch] Square, u32),
    Row { id: u32, #[dispatch] cell: Square },
    Plain(Square),
}

#[whichever::dispatch]
impl Any {
    #[dispatch(Module, Tagged, Row, Plain, field = 0)]
    pub fn side(&self) -> u32 {
        0
    }

    #[dispatch(Plain, Tagged, value, with = Some)]
    pub fn square(&self) -> Option<&Square> {
        None
    }
}

fn main() {}
