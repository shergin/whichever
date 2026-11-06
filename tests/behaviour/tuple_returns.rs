//! A tuple holding `Self` or an overridden associated type is converted
//! element by element: the tuple is taken apart and built again.

#[whichever::dispatchable]
pub trait Split: Sized {
    type Rest;
    fn split(&self) -> (Self, u32);
    fn halves(&self) -> (Self::Rest, Option<Self>);
}

#[derive(Debug, PartialEq)]
pub struct Even(pub u32);
#[derive(Debug, PartialEq)]
pub struct Odd(pub u32);

impl Split for Even {
    type Rest = u8;
    fn split(&self) -> (Self, u32) {
        (Even(self.0 / 2), self.0 / 2)
    }
    fn halves(&self) -> (Self::Rest, Option<Self>) {
        (1, Some(Even(self.0)))
    }
}

impl Split for Odd {
    type Rest = u16;
    fn split(&self) -> (Self, u32) {
        (Odd(self.0 - 1), 1)
    }
    fn halves(&self) -> (Self::Rest, Option<Self>) {
        (2, None)
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(Split, Rest = u32)]
pub enum Number {
    Even(Even),
    Odd(Odd),
}

// dash-num's shape: a wrapper's inherent method returning the wrapper
// and a flag.
pub struct U(pub u64);

impl U {
    pub fn mul_u64(self, by: u64) -> (Self, bool) {
        (U(self.0 * by), by == 0)
    }
}

#[whichever::dispatch]
pub struct Wrapped(U);

#[whichever::dispatch]
impl Wrapped {
    #[dispatch(*)]
    pub fn mul_u64(self, by: u64) -> (Self, bool);
}

#[test]
fn each_element_is_converted() {
    let (wrapped, zero) = Wrapped(U(3)).mul_u64(2);
    assert_eq!((wrapped.0 .0, zero), (6, false));
    assert_eq!(Number::Even(Even(4)).split(), (Number::Even(Even(2)), 2));
    assert_eq!(Number::Odd(Odd(3)).split(), (Number::Odd(Odd(2)), 1));
    assert_eq!(
        Number::Even(Even(4)).halves(),
        (1, Some(Number::Even(Even(4))))
    );
    assert_eq!(Number::Odd(Odd(3)).halves(), (2, None));
}
