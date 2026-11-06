//! An inherent block forwards on a struct as on an enum, under `*` or the
//! field's name.

pub struct Circle(pub u32);

impl Circle {
    pub fn radius(&self) -> u32 {
        self.0
    }
    pub fn describe(&self) -> String {
        format!("circle {}", self.0)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct U(pub u64);

impl U {
    pub fn wrapping_add(self, rhs: Self) -> Self {
        U(self.0.wrapping_add(rhs.0))
    }
    pub fn max_with(&self, other: &Self) -> Self {
        U(self.0.max(other.0))
    }
    pub fn take_from(&mut self, other: &mut Self) {
        self.0 += std::mem::take(&mut other.0);
    }
}

// Two values of a struct always hold the same field, so a `Self`
// argument is unwrapped to the field, as trait dispatch unwraps it.
#[derive(Debug, PartialEq, Clone, Copy)]
#[whichever::dispatch]
pub struct A(U);

#[whichever::dispatch]
impl A {
    #[dispatch(*)]
    pub fn wrapping_add(self, rhs: Self) -> Self;

    #[dispatch(*)]
    pub fn max_with(&self, other: &Self) -> Self;

    #[dispatch(*)]
    pub fn take_from(&mut self, other: &mut Self);
}

#[test]
fn a_self_argument_is_the_other_value_of_the_field() {
    assert_eq!(A(U(1)).wrapping_add(A(U(2))), A(U(3)));
    assert_eq!(A(U(1)).max_with(&A(U(5))), A(U(5)));
    let (mut a, mut b) = (A(U(1)), A(U(2)));
    a.take_from(&mut b);
    assert_eq!((a, b), (A(U(3)), A(U(0))));
}

#[whichever::dispatch]
pub struct Labelled {
    #[dispatch]
    shape: Circle,
    pub label: &'static str,
}

#[whichever::dispatch]
impl Labelled {
    #[dispatch(*)]
    pub fn radius(&self) -> u32;

    #[dispatch(shape, call = describe)]
    pub fn text(&self) -> String;
}

#[test]
fn the_field_answers() {
    let labelled = Labelled {
        shape: Circle(2),
        label: "a",
    };
    assert_eq!(labelled.radius(), 2);
    assert_eq!(labelled.text(), "circle 2");
    assert_eq!(labelled.label, "a");
}
