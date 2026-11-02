pub trait Swap {
    fn pricing_eq(&self, other: &Self) -> bool;
    fn with_rate(&self, rate: u64) -> Self;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Swap_a23a2b6355df660f_1_1 as Swap;
pub struct Pool(u64);
pub struct Book(u64);
impl Swap for Pool {
    fn pricing_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn with_rate(&self, rate: u64) -> Self {
        Pool(rate)
    }
}
impl Swap for Book {
    fn pricing_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn with_rate(&self, rate: u64) -> Self {
        Book(rate)
    }
}
pub enum SwapKind {
    Pool(Pool),
    Book(Book),
}
#[automatically_derived]
impl ::core::convert::From<Pool> for SwapKind {
    #[inline]
    fn from(value: Pool) -> Self {
        Self::Pool(value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Book> for SwapKind {
    #[inline]
    fn from(value: Book) -> Self {
        Self::Book(value)
    }
}
#[automatically_derived]
impl Swap for SwapKind {
    #[inline]
    #[allow(unreachable_patterns)]
    fn pricing_eq(&self, __argument0: &Self) -> bool {
        match (self, __argument0) {
            (SwapKind::Pool(__inner), SwapKind::Pool(__same0)) => {
                <Pool as Swap>::pricing_eq(__inner, __same0)
            }
            (SwapKind::Book(__inner), SwapKind::Book(__same0)) => {
                <Book as Swap>::pricing_eq(__inner, __same0)
            }
            _ => false,
        }
    }
    #[inline]
    fn with_rate(&self, __argument0: u64) -> Self {
        match self {
            SwapKind::Pool(__inner) => {
                SwapKind::Pool(<Pool as Swap>::with_rate(__inner, __argument0))
            }
            SwapKind::Book(__inner) => {
                SwapKind::Book(<Book as Swap>::with_rate(__inner, __argument0))
            }
        }
    }
}
fn main() {}
