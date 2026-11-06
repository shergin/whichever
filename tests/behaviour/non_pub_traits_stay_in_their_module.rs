// Declared before the traits' module on purpose: the enum reaches the
// carrier macro through the trait's re-export, not through textual order.
mod enums {
    use super::traits::{Area, Sealed};

    pub(super) struct Square(pub u32);
    pub(super) struct Circle(pub u32);

    impl Area for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
    }

    impl Area for Circle {
        fn area(&self) -> u32 {
            3 * self.0 * self.0
        }
    }

    impl Sealed for Square {}
    impl Sealed for Circle {
        fn sealed(&self) -> bool {
            false
        }
    }

    #[whichever::dispatch(Area)]
    #[whichever::dispatch(Sealed)]
    pub(super) enum Shape {
        Square(Square),
        Circle(Circle),
    }
}

mod traits {
    #[whichever::dispatchable]
    pub(crate) trait Area {
        fn area(&self) -> u32;
    }

    #[whichever::dispatchable]
    pub(super) trait Sealed {
        fn sealed(&self) -> bool {
            true
        }
    }
}

#[test]
fn a_crate_private_trait_dispatches_from_a_sibling_module() {
    use traits::{Area, Sealed};
    let square: enums::Shape = enums::Square(2).into();
    let circle: enums::Shape = enums::Circle(1).into();
    assert_eq!((square.area(), circle.area()), (4, 3));
    assert!(square.sealed());
    assert!(!circle.sealed());
}
