//! `with` names a function; a closure is reported as such.
struct Circle;

impl Circle {
    fn radius(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
enum AnyShape {
    Circle(Circle),
}

#[whichever::dispatch]
impl AnyShape {
    #[dispatch(Circle, with = |r| Some(r + 1))]
    fn radius(&self) -> Option<u32> {
        None
    }
}

fn main() {}
