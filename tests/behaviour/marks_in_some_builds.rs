//! A mark under `cfg_attr` is read in no build, so an item forwarded in
//! some builds only is written the ways the sentence names: a struct marks
//! its field in every build, with a bare attribute beside the trait's under
//! `cfg_attr`, and a variant or a method whose mark differs between builds
//! is written twice under `cfg`. `all()` and `not(any())` stand for a
//! predicate that holds and `any()` for one that does not, so both builds
//! are compiled here.

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

impl Square {
    pub fn describe(&self) -> String {
        format!("square {}", self.0)
    }
}

#[whichever::dispatch]
#[cfg_attr(all(), whichever::dispatch(Shape))]
pub struct Forwarded {
    #[dispatch]
    pub inner: Square,
    pub name: String,
}

#[cfg_attr(all(), whichever::dispatch(Shape))]
#[whichever::dispatch]
pub struct ForwardedTheOtherOrder {
    #[dispatch]
    pub inner: Square,
    pub name: String,
}

/// The build in which the trait is not forwarded: the field's mark is
/// taken off by the bare attribute, and the struct has no impl.
#[whichever::dispatch]
#[cfg_attr(any(), whichever::dispatch(Shape))]
pub struct NotForwarded {
    #[dispatch]
    pub inner: Square,
    pub name: String,
}

#[whichever::dispatch(Shape)]
pub enum Boxed {
    #[cfg(not(any()))]
    #[dispatch(deref)]
    Square(Box<Square>),
    #[cfg(any())]
    Square(Square),
}

#[whichever::dispatch(Shape)]
pub enum Plain {
    #[cfg(any())]
    #[dispatch(deref)]
    Square(Box<Square>),
    #[cfg(not(any()))]
    Square(Square),
}

#[whichever::dispatch]
pub enum Maybe {
    Square(Square),
    Empty,
}

#[whichever::dispatch]
impl Maybe {
    #[cfg(not(any()))]
    #[dispatch(Square)]
    pub fn describe(&self) -> String {
        String::from("nothing")
    }

    #[cfg(any())]
    pub fn describe(&self) -> String {
        String::from("nothing")
    }

    #[cfg(any())]
    #[dispatch(Square)]
    pub fn label(&self) -> String {
        String::from("nothing")
    }

    #[cfg(not(any()))]
    pub fn label(&self) -> String {
        String::from("nothing")
    }
}

#[test]
fn a_struct_marked_in_every_build_is_forwarded_where_the_attribute_holds() {
    let forwarded = Forwarded {
        inner: Square(3),
        name: String::from("three"),
    };
    assert_eq!(forwarded.area(), 9);
    let other = ForwardedTheOtherOrder {
        inner: Square(2),
        name: String::new(),
    };
    assert_eq!(other.area(), 4);
    let not = NotForwarded {
        inner: Square(1),
        name: String::from("one"),
    };
    assert_eq!((not.inner.area(), not.name.as_str()), (1, "one"));
}

#[test]
fn a_variant_written_twice_takes_the_mark_of_its_build() {
    assert_eq!(Boxed::Square(Box::new(Square(4))).area(), 16);
    assert_eq!(Plain::Square(Square(5)).area(), 25);
}

#[test]
fn a_method_written_twice_takes_the_mark_of_its_build() {
    let square = Maybe::Square(Square(6));
    assert_eq!(square.describe(), "square 6");
    assert_eq!(square.label(), "nothing");
    assert_eq!(Maybe::Empty.describe(), "nothing");
}
