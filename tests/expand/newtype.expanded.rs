pub trait Shape {
    fn area(&self) -> u32;
    fn grown(&self) -> Self
    where
        Self: Sized;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Shape_ff09a3ea4532a258_1_1 as Shape;
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
pub(crate) use __whichever_dispatch_AnyShape_cfb0fe2e33ee60c4_30_1 as AnyShape;
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
pub(crate) use __whichever_dispatch_Wrapped_8f2dfa023210b9ec_37_1 as Wrapped;
#[automatically_derived]
impl Shape for Wrapped {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            Wrapped(__inner) => <AnyShape as Shape>::area(__inner),
        }
    }
    #[inline]
    fn grown(&self) -> Self
    where
        Self: Sized,
    {
        match self {
            Wrapped(__inner) => Wrapped(<AnyShape as Shape>::grown(__inner)),
        }
    }
}
pub struct Labelled {
    shape: AnyShape,
    label: &'static str,
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Labelled_e7fec1347fe8a36e_40_1 as Labelled;
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
/// What needs no value is the field's type's: a function, `-> Self` built
/// around it, a provided function, and a generic associated type, whose
/// own `T` is renamed beside the struct's.
pub trait Kind {
    type Paired<T>;
    fn kind() -> &'static str;
    fn unit() -> Self;
    fn label() -> &'static str {
        "kind"
    }
    fn paired<T>(&self, with: T) -> Self::Paired<T>;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Kind_751754d095aa66bb_60_1 as Kind;
pub struct Generic<T>(T);
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Generic_8894e68c2e17ea23_71_1 as Generic;
#[automatically_derived]
impl<T> Kind for Generic<T>
where
    T: Kind,
{
    type Paired<T2> = <T as Kind>::Paired<T2>;
    #[inline]
    fn kind() -> &'static str {
        <T as Kind>::kind()
    }
    #[inline]
    fn unit() -> Self {
        Generic(<T as Kind>::unit())
    }
    #[inline]
    fn label() -> &'static str {
        <T as Kind>::label()
    }
    #[inline]
    fn paired<T2>(&self, with: T2) -> Self::Paired<T2> {
        match self {
            Generic(__inner) => <T as Kind>::paired(__inner, with),
        }
    }
}
pub struct Number(u128);
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Number_7fcf2cba61e7f7cf_74_1 as Number;
impl Number {
    pub fn from_u64(v: u64) -> Self {
        Self(<u128>::from(v))
    }
}
fn main() {}
