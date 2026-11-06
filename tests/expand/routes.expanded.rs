//! An arm reaches its target through a route: a field of the payload, the
//! marked field of a variant with several, or what a method returns; the
//! call is then qualified by the trait alone, since the target's type is
//! not the payload's.
pub trait Area {
    fn area(&self) -> u32;
    fn grow(&mut self, by: u32);
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Area_db9d8ab2eb5f6f7c_5_1 as Area;
pub struct Square(pub u32);
impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn grow(&mut self, by: u32) {
        self.0 += by;
    }
}
pub struct Account {
    pub base: Square,
    pub id: u32,
}
pub enum Any {
    Module(Account),
    Pair((u32, Square)),
    Tagged(Square, u32),
    Row { id: u32, cell: Square },
    Plain(Square),
}
#[automatically_derived]
impl ::core::convert::From<Account> for Any {
    #[inline]
    fn from(__value: Account) -> Self {
        Self::Module(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<(u32, Square)> for Any {
    #[inline]
    fn from(__value: (u32, Square)) -> Self {
        Self::Pair(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Square> for Any {
    #[inline]
    fn from(__value: Square) -> Self {
        Self::Plain(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Any_facda45d93460f69_27_1 as Any;
#[automatically_derived]
impl Area for Any {
    #[inline]
    fn area(&self) -> u32 {
        match self {
            Any::Module(__inner) => <_ as Area>::area(&__inner.base),
            Any::Pair(__inner) => <_ as Area>::area(&__inner.1),
            Any::Tagged(__inner, ..) => <Square as Area>::area(__inner),
            Any::Row { cell: __inner, .. } => <Square as Area>::area(__inner),
            Any::Plain(__inner) => <Square as Area>::area(__inner),
        }
    }
    #[inline]
    fn grow(&mut self, by: u32) {
        match self {
            Any::Module(__inner) => <_ as Area>::grow(&mut __inner.base, by),
            Any::Pair(__inner) => <_ as Area>::grow(&mut __inner.1, by),
            Any::Tagged(__inner, ..) => <Square as Area>::grow(__inner, by),
            Any::Row { cell: __inner, .. } => <Square as Area>::grow(__inner, by),
            Any::Plain(__inner) => <Square as Area>::grow(__inner, by),
        }
    }
}
impl Any {
    pub fn side(&self) -> u32 {
        match self {
            Self::Module(__inner) => __inner.base.0,
            Self::Tagged(__inner, ..) => __inner.0,
            Self::Row { cell: __inner, .. } => __inner.0,
            Self::Plain(__inner) => __inner.0,
            _ => 0,
        }
    }
    pub fn square(&self) -> Option<&Square> {
        match self {
            Self::Plain(__inner) => Some(__inner),
            Self::Tagged(__inner, ..) => Some(__inner),
            _ => None,
        }
    }
}
fn main() {}
