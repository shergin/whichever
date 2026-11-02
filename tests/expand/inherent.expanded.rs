pub struct Circle(pub u32);
pub struct Square(pub u32);
impl Circle {
    pub fn radius(&self) -> u32 {
        self.0
    }
    pub fn describe(&self) -> String {
        ::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("circle {0}", self.0))
        })
    }
}
impl Square {
    pub fn describe(&self) -> String {
        ::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("square {0}", self.0))
        })
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
impl ::core::convert::TryFrom<AnyShape> for Circle {
    type Error = AnyShape;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(__value: AnyShape) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            AnyShape::Circle(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[automatically_derived]
impl ::core::convert::From<Square> for AnyShape {
    #[inline]
    fn from(__value: Square) -> Self {
        Self::Square(__value)
    }
}
#[automatically_derived]
impl ::core::convert::TryFrom<AnyShape> for Square {
    type Error = AnyShape;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(__value: AnyShape) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            AnyShape::Square(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnyShape_ecc323c5dd12c245_19_1 as AnyShape;
impl AnyShape {
    /// Only circles have a radius.
    pub fn radius(&self) -> Option<u32> {
        match self {
            Self::Circle(__inner) => Some(__inner.radius()),
            _ => None,
        }
    }
    pub fn describe(&self) -> String {
        match self {
            Self::Circle(__inner) => __inner.describe(),
            Self::Square(__inner) => __inner.describe(),
        }
    }
}
fn main() {}
