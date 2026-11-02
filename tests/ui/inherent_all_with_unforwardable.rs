struct IntBounds(u32);

impl IntBounds {
    fn len(&self) -> usize {
        1
    }
}

#[whichever::dispatch]
enum Bounds {
    Int(IntBounds),
    NoOrder,
}

#[whichever::dispatch]
impl Bounds {
    #[dispatch(*)]
    fn len(&self) -> usize;
}

fn main() {}
