//! An associated type of one of the enum's type parameters may be any type,
//! the enum itself included, so `From<S::Out>` would overlap `From<T> for
//! T` even with one variant; each spelling is reported on its payload.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
}

pub trait Source {
    type Out: Shape;
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Shape)]
pub enum Plain<S: Source> {
    Out(S::Out),
    Square(Square),
}

#[whichever::dispatch(Shape)]
pub enum Qualified<S: Source> {
    Out(<S as Source>::Out),
}

fn main() {}
