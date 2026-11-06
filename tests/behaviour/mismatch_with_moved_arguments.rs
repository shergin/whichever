//! A `mismatch` written on the trait method may use a `Self` argument the
//! scrutinee moved: the fallback arm binds it again under its own name.

#[whichever::dispatchable]
pub trait Merge: Sized {
    #[dispatch(mismatch = other)]
    fn merge(self, other: Self) -> Self;
    #[dispatch(mismatch = {
        other.reset();
        false
    })]
    fn absorb(&mut self, other: &mut Self) -> bool;
    fn reset(&mut self);
}

#[derive(Debug, PartialEq)]
pub struct A(pub u8);
#[derive(Debug, PartialEq)]
pub struct B(pub u8);

impl Merge for A {
    fn merge(self, other: Self) -> Self {
        A(self.0 + other.0)
    }
    fn absorb(&mut self, other: &mut Self) -> bool {
        self.0 += other.0;
        other.0 = 0;
        true
    }
    fn reset(&mut self) {
        self.0 = 0;
    }
}

impl Merge for B {
    fn merge(self, other: Self) -> Self {
        B(self.0 + other.0)
    }
    fn absorb(&mut self, other: &mut Self) -> bool {
        self.0 += other.0;
        other.0 = 0;
        true
    }
    fn reset(&mut self) {
        self.0 = 0;
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(Merge)]
pub enum Any {
    A(A),
    B(B),
}

#[test]
fn different_variants_take_the_fallback_with_the_arguments() {
    assert_eq!(Any::A(A(1)).merge(Any::A(A(2))), Any::A(A(3)));
    assert_eq!(Any::A(A(1)).merge(Any::B(B(5))), Any::B(B(5)));
    let mut a = Any::A(A(1));
    let mut b = Any::B(B(5));
    assert!(!a.absorb(&mut b));
    assert_eq!(b, Any::B(B(0)));
    let mut other = Any::A(A(4));
    assert!(a.absorb(&mut other));
    assert_eq!((a, other), (Any::A(A(5)), Any::A(A(0))));
}
