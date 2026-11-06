//! An option naming nothing in the trait is reported beside the impl, which
//! is right without it, so the calls around the enum resolve and the one
//! sentence stands alone.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape, Eror = u8, mismatch(congruent = false))]
enum Any {
    Square(Square),
}

fn main() {
    let any = Any::Square(Square(2));
    let _ = any.area();
}
