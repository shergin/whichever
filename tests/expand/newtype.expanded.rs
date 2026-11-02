pub trait Shape {
    fn area(&self) -> u32;
    fn grown(&self) -> Self
    where
        Self: Sized;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Shape_37aae1dd40d611c4_1_1 as Shape;
pub struct Circle(pub u32);
pub struct Square(pub u32);
impl Shape for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
    fn grown(&self) -> Self {
        Circle(self.0 + 1)
    }
}
impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn grown(&self) -> Self {
        Square(self.0 + 1)
    }
}
enum AnyShape {
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
pub(crate) use __whichever_enum_AnyShape_24d80d085656751b_30_1 as AnyShape;
#[automatically_derived]
impl Shape for AnyShape {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            AnyShape::Circle(__inner) => <Circle as Shape>::area(__inner),
            AnyShape::Square(__inner) => <Square as Shape>::area(__inner),
        }
    }
    #[inline]
    fn grown(&self) -> Self
    where
        Self: Sized,
    {
        match self {
            AnyShape::Circle(__inner) => {
                AnyShape::Circle(<Circle as Shape>::grown(__inner))
            }
            AnyShape::Square(__inner) => {
                AnyShape::Square(<Square as Shape>::grown(__inner))
            }
        }
    }
}
/// The public wrapper over the private enum.
pub struct Wrapped(AnyShape);
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_enum_Wrapped_ea4ff03bb5df9e0c_37_1 as Wrapped;
#[automatically_derived]
impl Shape for Wrapped {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            Wrapped(__inner, ..) => <AnyShape as Shape>::area(__inner),
        }
    }
    #[inline]
    fn grown(&self) -> Self
    where
        Self: Sized,
    {
        match self {
            Wrapped(__inner, ..) => Wrapped(<AnyShape as Shape>::grown(__inner)),
        }
    }
}
pub struct Labelled {
    shape: AnyShape,
    label: &'static str,
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_enum_Labelled_f80b87cee4c64ca9_40_1 as Labelled;
impl Shape for Labelled {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            Labelled { shape: __inner, .. } => <AnyShape as Shape>::area(__inner),
        }
    }
    fn grown(&self) -> Self {
        Labelled {
            shape: self.shape.grown(),
            label: self.label,
        }
    }
}
fn main() {}
