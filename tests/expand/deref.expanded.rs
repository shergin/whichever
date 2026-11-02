pub trait Area {
    fn area(&self) -> u32;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Area_91d2a763e686069f_1_1 as Area;
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
    fn from(value: Box<Square>) -> Self {
        Self::Boxed(value)
    }
}
#[automatically_derived]
impl<'a> ::core::convert::From<&'a Square> for Shape<'a> {
    #[inline]
    fn from(value: &'a Square) -> Self {
        Self::Borrowed(value)
    }
}
#[automatically_derived]
impl<'a> ::core::convert::From<Square> for Shape<'a> {
    #[inline]
    fn from(value: Square) -> Self {
        Self::Plain(value)
    }
}
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
