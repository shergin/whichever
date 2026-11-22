//! Two fields marked bare are one error, and the impl is still written,
//! so the coercion below finds it.
#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

struct Circle(u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Shape)]
struct Pair {
    #[dispatch]
    left: Circle,
    #[dispatch]
    right: Circle,
}

fn boxed(pair: Pair) -> Box<dyn Shape> {
    Box::new(pair)
}

fn main() {
    let _ = boxed;
}
