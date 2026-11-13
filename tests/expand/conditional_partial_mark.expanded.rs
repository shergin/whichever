//! Methods a partial block marks with `#[dispatch(..)]` under `#[cfg]`: a
//! mark is the block's where its `cfg` holds, and the method is forwarded to
//! every variant where it does not. `cargo expand` evaluates the `cfg`, and
//! `test` is off there, so `name` and `kind`, marked under `not(test)`, show
//! the build where the `cfg` holds: `name` is left to the default and `kind`
//! takes its body for `Square`. `label` and `size`, marked under `test`,
//! show the build where it does not: both are forwarded to every variant.
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
    fn label(&self) -> &'static str {
        "shape"
    }
    fn kind(&self) -> &'static str {
        "shape"
    }
    fn size(&self) -> &'static str {
        "shape"
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Shape_bf27099cd7af3c55_8_1 as Shape;
pub struct Circle;
impl Shape for Circle {
    fn area(&self) -> u32 {
        3
    }
}
pub struct Square;
impl Shape for Square {
    fn area(&self) -> u32 {
        4
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
pub(crate) use __whichever_dispatch_AnyShape_ecc323c5dd12c245_41_1 as AnyShape;
impl Shape for AnyShape {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            AnyShape::Circle(__inner) => <Circle as Shape>::area(__inner),
            AnyShape::Square(__inner) => <Square as Shape>::area(__inner),
        }
    }
    #[inline]
    fn label(&self) -> &'static str {
        match self {
            AnyShape::Circle(__inner) => <Circle as Shape>::label(__inner),
            AnyShape::Square(__inner) => <Square as Shape>::label(__inner),
        }
    }
    #[inline]
    fn kind(&self) -> &'static str {
        match self {
            AnyShape::Circle(__inner) => <Circle as Shape>::kind(__inner),
            _ => "body",
        }
    }
    #[inline]
    fn size(&self) -> &'static str {
        match self {
            AnyShape::Circle(__inner) => <Circle as Shape>::size(__inner),
            AnyShape::Square(__inner) => <Square as Shape>::size(__inner),
        }
    }
}
fn main() {}
