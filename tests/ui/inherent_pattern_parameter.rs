struct Circle;

impl Circle {
    fn offset(&self, x: u32, y: u32) -> u32 {
        x + y
    }
}

#[whichever::dispatch]
enum AnyShape {
    Circle(Circle),
}

#[whichever::dispatch]
impl AnyShape {
    #[dispatch(*)]
    fn offset(&self, (x, y): (u32, u32)) -> u32;
}

fn main() {}
