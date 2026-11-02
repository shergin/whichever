pub trait Shape {
    fn congruent(&self, other: &Self) -> bool;
    fn scaled(&self, factor: u32) -> Self;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Shape_1d37b16f2690c8b0_1_1 as Shape;
pub struct Circle(u32);
pub struct Square(u32);
impl Shape for Circle {
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn scaled(&self, factor: u32) -> Self {
        Circle(self.0 * factor)
    }
}
impl Shape for Square {
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn scaled(&self, factor: u32) -> Self {
        Square(self.0 * factor)
    }
}
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}
#[automatically_derived]
impl ::core::convert::From<Circle> for AnyShape {
    #[inline]
    fn from(__value: Circle) -> Self {
        Self::Circle(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Square> for AnyShape {
    #[inline]
    fn from(__value: Square) -> Self {
        Self::Square(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_enum_AnyShape_3091d3476d2b3a0e_29_1 as AnyShape;
#[automatically_derived]
impl Shape for AnyShape {
    #[inline]
    #[allow(unreachable_patterns)]
    fn congruent(&self, __argument0: &Self) -> bool {
        match (self, __argument0) {
            (AnyShape::Circle(__inner), AnyShape::Circle(__same0)) => {
                <Circle as Shape>::congruent(__inner, __same0)
            }
            (AnyShape::Square(__inner), AnyShape::Square(__same0)) => {
                <Square as Shape>::congruent(__inner, __same0)
            }
            _ => false,
        }
    }
    #[inline]
    fn scaled(&self, __argument0: u32) -> Self {
        match self {
            AnyShape::Circle(__inner) => {
                AnyShape::Circle(<Circle as Shape>::scaled(__inner, __argument0))
            }
            AnyShape::Square(__inner) => {
                AnyShape::Square(<Square as Shape>::scaled(__inner, __argument0))
            }
        }
    }
}
fn main() {}
