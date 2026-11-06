#[whichever::dispatchable]
trait Area {
    fn area(&self) -> u32;
}

struct Square(u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

/// A test-only variant that is compiled out is no error.
#[whichever::dispatch(Area)]
enum Shape {
    Square(Square),
    #[cfg(any())]
    Mock,
}

struct IntBounds(u32, u32);
struct TextBounds(&'static str);

impl IntBounds {
    fn len(&self) -> usize {
        (self.1 - self.0) as usize
    }
}

impl TextBounds {
    fn len(&self) -> usize {
        self.0.len()
    }
}

/// A data enum: the odd variants are left to the inherent fallbacks.
#[whichever::dispatch]
enum Bounds {
    Int(IntBounds),
    Text(TextBounds),
    NoOrder,
    Pair(u8, u8),
}

#[whichever::dispatch]
impl Bounds {
    #[dispatch(Int, Text)]
    fn len(&self) -> usize {
        0
    }
}

#[test]
fn odd_variants_take_the_fallback_and_cfg_out_variants_are_no_error() {
    let shape: Shape = Square(3).into();
    assert_eq!(shape.area(), 9);
    let bounds: Bounds = IntBounds(2, 5).into();
    assert_eq!(bounds.len(), 3);
    assert_eq!(Bounds::Text(TextBounds("ab")).len(), 2);
    assert_eq!((Bounds::NoOrder.len(), Bounds::Pair(1, 2).len()), (0, 0));
}
