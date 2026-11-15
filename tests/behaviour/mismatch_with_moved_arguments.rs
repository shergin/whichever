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

/// A partial block's fallback arm binds a moved `Self` argument again only
/// where the `mismatch` names it, so one it does not name is no unused
/// variable in the user's crate, where a partial block's impl, unlike the
/// enum's, is not automatically derived.
#[deny(unused_variables)]
mod a_partial_block_whose_mismatch_names_no_argument {
    #[whichever::dispatchable]
    pub trait Merge: Sized {
        fn merge(self, other: Self, by: u64) -> u64;
        fn absorb(&mut self, other: &mut Self) -> bool;
    }

    pub struct A(pub u64);
    pub struct B(pub u64);

    impl Merge for A {
        fn merge(self, other: Self, by: u64) -> u64 {
            self.0 + other.0 + by
        }
        fn absorb(&mut self, other: &mut Self) -> bool {
            self.0 += other.0;
            true
        }
    }

    impl Merge for B {
        fn merge(self, other: Self, by: u64) -> u64 {
            self.0 * other.0 * by
        }
        fn absorb(&mut self, other: &mut Self) -> bool {
            self.0 *= other.0;
            true
        }
    }

    #[whichever::dispatch]
    pub enum Any {
        A(A),
        B(B),
    }

    #[whichever::dispatch(mismatch(merge = by, absorb = false))]
    impl Merge for Any {}

    #[whichever::dispatch]
    pub enum Starred {
        A(A),
        B(B),
    }

    #[whichever::dispatch(mismatch(merge = by, absorb = false))]
    impl Merge for Starred {
        #[dispatch(*)]
        fn merge(self, other: Self, by: u64) -> u64;
        #[dispatch(*)]
        fn absorb(&mut self, other: &mut Self) -> bool;
    }

    #[test]
    fn the_fallback_answers_without_the_argument() {
        assert_eq!(Any::A(A(1)).merge(Any::A(A(2)), 3), 6);
        assert_eq!(Any::A(A(1)).merge(Any::B(B(2)), 3), 3);
        assert!(!Any::A(A(1)).absorb(&mut Any::B(B(2))));
        assert!(Any::B(B(1)).absorb(&mut Any::B(B(2))));
        assert_eq!(Starred::B(B(2)).merge(Starred::B(B(3)), 4), 24);
        assert_eq!(Starred::B(B(2)).merge(Starred::A(A(3)), 4), 4);
        assert!(!Starred::A(A(1)).absorb(&mut Starred::B(B(2))));
    }
}

/// Where the `mismatch` does name the moved argument, the fallback arm
/// binds it again, a name a format string captures included.
#[deny(unused_variables)]
mod a_partial_block_whose_mismatch_names_the_argument {
    #[whichever::dispatchable]
    pub trait Describe: Sized {
        fn describe(self, other: Self) -> String;
        fn take(&mut self, other: &mut Self) -> u64;
        fn reset(&mut self);
    }

    #[derive(Debug)]
    pub struct A(pub u64);
    #[derive(Debug)]
    pub struct B(pub u64);

    impl Describe for A {
        fn describe(self, other: Self) -> String {
            format!("A{}{}", self.0, other.0)
        }
        fn take(&mut self, other: &mut Self) -> u64 {
            core::mem::take(&mut other.0)
        }
        fn reset(&mut self) {
            self.0 = 0;
        }
    }

    impl Describe for B {
        fn describe(self, other: Self) -> String {
            format!("B{}{}", self.0, other.0)
        }
        fn take(&mut self, other: &mut Self) -> u64 {
            core::mem::take(&mut other.0)
        }
        fn reset(&mut self) {
            self.0 = 0;
        }
    }

    #[derive(Debug)]
    #[whichever::dispatch]
    pub enum Any {
        A(A),
        B(B),
    }

    #[whichever::dispatch(mismatch(
        describe = format!("{other:?}"),
        take = {
            other.reset();
            7
        },
    ))]
    impl Describe for Any {}

    #[test]
    fn the_fallback_sees_the_argument() {
        assert_eq!(Any::A(A(1)).describe(Any::A(A(2))), "A12");
        assert_eq!(Any::A(A(1)).describe(Any::B(B(2))), "B(B(2))");
        let mut other = Any::B(B(5));
        assert_eq!(Any::A(A(1)).take(&mut other), 7);
        assert!(matches!(other, Any::B(B(0))));
        let mut same = Any::A(A(5));
        assert_eq!(Any::A(A(1)).take(&mut same), 5);
    }
}
