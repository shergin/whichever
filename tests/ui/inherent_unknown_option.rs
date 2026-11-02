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
    #[dispatch(*, wrap = Some)]
    fn radius(&self) -> u32;
}

fn main() {}
