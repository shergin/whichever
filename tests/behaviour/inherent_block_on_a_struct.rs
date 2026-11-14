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

/// `*` forwards to the field marked bare; a field's name forwards to that
/// field, marked or not.
#[whichever::dispatch]
pub struct Pair {
    #[dispatch]
    left: Circle,
    right: Circle,
}

#[whichever::dispatch]
impl Pair {
    #[dispatch(*)]
    pub fn radius(&self) -> u32;

    #[dispatch(right, call = radius)]
    pub fn right_radius(&self) -> u32;

    #[dispatch(left, call = describe, with = Some)]
    pub fn left_text(&self) -> Option<String>;
}

#[test]
fn a_named_field_answers_and_star_is_the_bare_one() {
    let pair = Pair {
        left: Circle(1),
        right: Circle(2),
    };
    assert_eq!(pair.radius(), 1);
    assert_eq!(pair.right_radius(), 2);
    assert_eq!(pair.left_text(), Some("circle 1".to_owned()));
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Handle(pub u64);

/// A marked field beside another: `value` reads the field the pattern
/// binds where the return type asks for a value and hands the binding over
/// where it asks for a reference, for the marked field and for one named on
/// the method alike.
#[whichever::dispatch]
pub struct Slot {
    #[dispatch]
    handle: Handle,
    label: String,
}

#[whichever::dispatch]
impl Slot {
    #[dispatch(*, value)]
    pub fn handle(&self) -> Handle;

    #[dispatch(*, value)]
    pub fn handle_ref(&self) -> &Handle;

    #[dispatch(*, value)]
    pub fn handle_mut(&mut self) -> &mut Handle;

    #[dispatch(*, value, with = Some)]
    pub fn some_handle(&self) -> Option<Handle>;

    #[dispatch(label, value)]
    pub fn label(&self) -> &String;

    // An opaque return takes the binding as it stands, so a field that is
    // not `Copy` is still handed over by reference.
    #[dispatch(label, value)]
    pub fn shown(&self) -> impl core::fmt::Display + '_;
}

/// A struct's only field, read by value through the binding.
#[whichever::dispatch]
pub struct Wrapped(Handle);

#[whichever::dispatch]
impl Wrapped {
    #[dispatch(*, value)]
    pub fn get(&self) -> Handle;
}

#[test]
fn a_field_is_read_by_value_and_borrowed_by_reference() {
    let mut slot = Slot {
        handle: Handle(7),
        label: "seven".to_owned(),
    };
    assert_eq!(slot.handle(), Handle(7));
    assert!(std::ptr::eq(slot.handle_ref(), &slot.handle));
    assert!(std::ptr::eq(slot.label(), &slot.label));
    assert_eq!(slot.some_handle(), Some(Handle(7)));
    assert_eq!(slot.shown().to_string(), "seven");
    slot.handle_mut().0 += 1;
    assert_eq!(slot.handle(), Handle(8));
    assert_eq!(Wrapped(Handle(9)).get(), Handle(9));
}
