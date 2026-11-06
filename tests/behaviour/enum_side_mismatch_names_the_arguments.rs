//! A `mismatch` written on the enum may use the method's parameters and
//! `self`, as one on the trait method may: the fallback arm's bindings
//! resolve where the expression was written. A `&mut self` or by-value
//! receiver is moved into the `match`, so `self` is there to use for `&self`.

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn congruent(&self, other: &Self) -> bool;
    fn absorb(&mut self, other: Self) -> bool;
}

pub struct Circle(pub u32);
pub struct Square(pub u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        4 * self.0 * self.0
    }
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn absorb(&mut self, other: Self) -> bool {
        self.0 += other.0;
        true
    }
}

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn absorb(&mut self, other: Self) -> bool {
        self.0 += other.0;
        true
    }
}

#[whichever::dispatch(Shape, mismatch(
    congruent = other.area() == self.area(),
    absorb = other.area() < 9,
))]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}

#[test]
fn the_expression_sees_self_and_the_arguments() {
    let circle = AnyShape::Circle(Circle(1));
    assert!(circle.congruent(&AnyShape::Square(Square(2))));
    assert!(!circle.congruent(&AnyShape::Square(Square(3))));
    assert!(circle.congruent(&AnyShape::Circle(Circle(1))));
    let mut square = AnyShape::Square(Square(3));
    assert!(square.absorb(AnyShape::Circle(Circle(1))));
    assert!(!square.absorb(AnyShape::Circle(Circle(2))));
    assert!(square.absorb(AnyShape::Square(Square(1))));
    assert_eq!(square.area(), 16);
}
