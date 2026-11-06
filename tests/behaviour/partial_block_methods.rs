//! A method of a partial block names the variants it forwards to, and its
//! body answers for the rest, two different variants of a `Self` argument
//! included; `#[dispatch(default)]` leaves a method to the trait's default.

use std::fmt::{self, Display, Formatter};

#[whichever::dispatchable]
pub trait Priced {
    fn price(&self) -> u32;
    fn same_price(&self, other: &Self) -> bool;
    fn label(&self) -> &'static str {
        "priced"
    }
    fn currency(&self) -> &'static str {
        "EUR"
    }
}

pub struct Book(pub u32);
pub struct Pen(pub u32);

impl Priced for Book {
    fn price(&self) -> u32 {
        self.0
    }
    fn same_price(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn label(&self) -> &'static str {
        "book"
    }
}

impl Priced for Pen {
    fn price(&self) -> u32 {
        self.0
    }
    fn same_price(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn currency(&self) -> &'static str {
        "USD"
    }
}

impl Display for Book {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "book at {}", self.0)
    }
}

#[whichever::dispatch]
pub enum Item {
    Book(Book),
    Pen(Pen),
    Gift,
}

#[whichever::dispatch]
impl Priced for Item {
    /// A gift is free.
    #[dispatch(Book, Pen)]
    fn price(&self) -> u32 {
        0
    }
    /// Two different kinds, or a gift, never share a price.
    #[dispatch(Book, Pen)]
    fn same_price(&self, other: &Self) -> bool {
        false
    }
    #[dispatch(Book, Pen)]
    fn label(&self) -> &'static str {
        "gift"
    }
    /// Whatever the payloads say, the enum's currency is the trait's.
    #[dispatch(default)]
    fn currency(&self) -> &'static str;
}

// A foreign trait with a unit variant answered by hand.
#[whichever::dispatch]
impl whichever::foreign::Display for Item {
    #[dispatch(Book)]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("something else")
    }
}

#[test]
fn the_named_variants_forward_and_the_body_answers_for_the_rest() {
    let book = Item::Book(Book(10));
    let pen = Item::Pen(Pen(10));
    assert_eq!((book.price(), pen.price(), Item::Gift.price()), (10, 10, 0));
    assert!(book.same_price(&Item::Book(Book(10))));
    assert!(!book.same_price(&pen));
    assert!(!Item::Gift.same_price(&Item::Gift));
    assert_eq!(
        (book.label(), pen.label(), Item::Gift.label()),
        ("book", "priced", "gift")
    );
    assert_eq!((book.currency(), pen.currency()), ("EUR", "EUR"));
    assert_eq!(book.to_string(), "book at 10");
    assert_eq!(Item::Gift.to_string(), "something else");
}
