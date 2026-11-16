//! A variant whose one field is named and marked, beside a tuple variant:
//! a method returning `Self` rebuilds it by its braces, `Any::Named { inner:
//! .. }`, where the tuple variant is rebuilt by its name, and only the tuple
//! variant converts from its payload.
pub trait Shape {
    fn radius(&self) -> u32;
    fn doubled(self) -> Self;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Shape_64d834d42c120ab8_5_1 as Shape;
pub struct Circle(u32);
impl Shape for Circle {
    fn radius(&self) -> u32 {
        self.0
    }
    fn doubled(self) -> Self {
        Circle(self.0 * 2)
    }
}
pub enum Any {
    Named { inner: Circle },
    Plain(Circle),
}
#[automatically_derived]
impl ::core::convert::From<Circle> for Any {
    #[inline]
    fn from(__value: Circle) -> Self {
        Self::Plain(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Any_74101ed5d72521d7_22_1 as Any;
#[automatically_derived]
impl Shape for Any {
    #[inline]
    fn radius(&self) -> u32 {
        match self {
            Any::Named { inner: __inner } => <Circle as Shape>::radius(__inner),
            Any::Plain(__inner) => <Circle as Shape>::radius(__inner),
        }
    }
    #[inline]
    fn doubled(self) -> Self {
        match self {
            Any::Named { inner: __inner } => {
                Any::Named {
                    inner: <Circle as Shape>::doubled(__inner),
                }
            }
            Any::Plain(__inner) => Any::Plain(<Circle as Shape>::doubled(__inner)),
        }
    }
}
fn main() {}
