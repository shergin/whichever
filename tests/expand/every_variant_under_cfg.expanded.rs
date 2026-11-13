use std::pin::Pin;
pub trait Shape {
    fn area(&self) -> u32;
    fn grow(&mut self, by: u32);
    fn into_area(self) -> u32;
    fn poll_area(self: Pin<&mut Self>) -> u32;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Shape_4505e93d527cea17_3_1 as Shape;
pub struct Circle(pub u32);
impl Shape for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
    fn grow(&mut self, by: u32) {
        self.0 += by;
    }
    fn into_area(self) -> u32 {
        self.area()
    }
    fn poll_area(self: Pin<&mut Self>) -> u32 {
        self.area()
    }
}
pub enum AnyShape {
    Circle(Circle),
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnyShape_4c62e0304ca14750_30_1 as AnyShape;
#[automatically_derived]
impl Shape for AnyShape {
    #[inline]
    fn area(&self) -> u32 {
        match *self {
            AnyShape::Circle(ref __inner) => <Circle as Shape>::area(__inner),
        }
    }
    #[inline]
    fn grow(&mut self, by: u32) {
        match *self {
            AnyShape::Circle(ref mut __inner) => <Circle as Shape>::grow(__inner, by),
        }
    }
    #[inline]
    fn into_area(self) -> u32 {
        match self {
            AnyShape::Circle(__inner) => <Circle as Shape>::into_area(__inner),
        }
    }
    #[inline]
    fn poll_area(self: Pin<&mut Self>) -> u32 {
        match *::core::pin::Pin::get_mut(self) {
            AnyShape::Circle(ref mut __inner) => {
                <Circle as Shape>::poll_area(::core::pin::Pin::new(__inner))
            }
        }
    }
}
fn main() {}
