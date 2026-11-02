//! A rejected `#[dispatch]` on an impl block keeps the block, so its other
//! methods resolve and the one error stands alone.
struct Circle;

#[whichever::dispatch]
enum Shape {
    Circle(Circle),
}

#[whichever::dispatch(Shape)]
impl Shape {
    pub fn plain(&self) -> u32 {
        2
    }
}

fn main() {
    let _ = Shape::Circle(Circle).plain();
}
