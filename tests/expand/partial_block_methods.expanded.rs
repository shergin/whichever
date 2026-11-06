//! A method of a partial block names the variants it forwards to, with its
//! body for the rest; `#[dispatch(default)]` leaves one to the trait.
pub trait Priced {
    fn price(&self) -> u32;
    fn label(&self) -> &'static str {
        "priced"
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Priced_6b6cde0b7258db8c_3_1 as Priced;
pub struct Book(pub u32);
impl Priced for Book {
    fn price(&self) -> u32 {
        self.0
    }
}
pub enum Item {
    Book(Book),
    Gift,
}
#[automatically_derived]
impl ::core::convert::From<Book> for Item {
    #[inline]
    fn from(__value: Book) -> Self {
        Self::Book(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Item_dd37dacf337ff33c_19_1 as Item;
impl Priced for Item {
    #[inline]
    fn price(&self) -> u32 {
        match self {
            Item::Book(__inner) => <Book as Priced>::price(__inner),
            _ => 0,
        }
    }
}
fn main() {}
