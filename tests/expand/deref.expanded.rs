pub trait Area {
    fn area(&self) -> u32;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Area_435c6d10ecc02ae3_1_1 as Area;
pub struct Square(pub u32);
impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}
pub enum Shape<'a> {
    Boxed(Box<Square>),
    Borrowed(&'a Square),
    Plain(Square),
}
#[automatically_derived]
impl<'a> ::core::convert::From<Box<Square>> for Shape<'a> {
    #[inline]
    fn from(__value: Box<Square>) -> Self {
        Self::Boxed(__value)
    }
}
#[automatically_derived]
impl<'a> ::core::convert::From<&'a Square> for Shape<'a> {
    #[inline]
    fn from(__value: &'a Square) -> Self {
        Self::Borrowed(__value)
    }
}
#[automatically_derived]
impl<'a> ::core::convert::From<Square> for Shape<'a> {
    #[inline]
    fn from(__value: Square) -> Self {
        Self::Plain(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Shape_91a70a89b1e41dce_14_1 as Shape;
#[automatically_derived]
impl<'a> Area for Shape<'a> {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            Shape::Boxed(__inner) => {
                <<Box<Square> as ::core::ops::Deref>::Target as Area>::area(&**__inner)
            }
            Shape::Borrowed(__inner) => {
                <<&'a Square as ::core::ops::Deref>::Target as Area>::area(&**__inner)
            }
            Shape::Plain(__inner) => <Square as Area>::area(__inner),
        }
    }
}
fn main() {}
