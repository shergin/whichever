//! A method taking `self: Box<Self>` cannot be forwarded through a struct
//! whose field holds its payload plainly: there is no box to hand over. The
//! sentence names the struct, and the way around is a boxed field.
#[whichever::dispatchable]
trait Consume {
    fn consume(self: Box<Self>) -> u32;
}

struct Square(u32);

impl Consume for Square {
    fn consume(self: Box<Self>) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Consume)]
struct Wrapper(Square);

fn main() {}
