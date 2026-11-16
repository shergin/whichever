//! A method left to the trait's default carries one `#[dispatch(..)]`, as
//! every marked method of a partial block does.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
    fn sides(&self) -> u32 {
        4
    }
}

struct Square(u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch]
enum Any {
    Square(Square),
}

#[whichever::dispatch]
impl Shape for Any {
    #[dispatch(default)]
    #[dispatch(*)]
    fn sides(&self) -> u32;
}

fn main() {}
