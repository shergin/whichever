// One `macro_rules!` emits the same trait text in two modules. Neither
// the text nor the call site of `#[dispatchable]`, which lies in the
// macro's body for both expansions, tells them apart; the tokens the
// invocation supplies, the trait's name or a method's, do.
macro_rules! area_trait {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub trait $name {
            fn area(&self) -> u64;
        }
    };
}

macro_rules! measure_trait {
    ($method:ident) => {
        #[whichever::dispatchable]
        pub trait Measure {
            fn $method(&self) -> u64;
        }
    };
}

mod one {
    area_trait!(
        #[whichever::dispatchable]
        Area
    );
    measure_trait!(perimeter);

    pub struct Square(pub u64);

    impl Area for Square {
        fn area(&self) -> u64 {
            self.0 * self.0
        }
    }

    impl Measure for Square {
        fn perimeter(&self) -> u64 {
            self.0 * 4
        }
    }

    #[whichever::dispatch(Area)]
    #[dispatch(Measure)]
    pub enum Shape {
        Square(Square),
    }
}

mod two {
    area_trait!(
        #[whichever::dispatchable]
        Area
    );
    measure_trait!(perimeter);

    pub struct Rectangle(pub u64, pub u64);

    impl Area for Rectangle {
        fn area(&self) -> u64 {
            self.0 * self.1
        }
    }

    impl Measure for Rectangle {
        fn perimeter(&self) -> u64 {
            (self.0 + self.1) * 2
        }
    }

    #[whichever::dispatch(Area)]
    #[dispatch(Measure)]
    pub enum Shape {
        Rectangle(Rectangle),
    }
}

#[test]
fn exported_macros_do_not_collide() {
    use one::{Area as _, Measure as _};
    use two::{Area as _, Measure as _};
    let square = one::Shape::from(one::Square(3));
    let rectangle = two::Shape::from(two::Rectangle(2, 5));
    assert_eq!((square.area(), square.perimeter()), (9, 12));
    assert_eq!((rectangle.area(), rectangle.perimeter()), (10, 14));
}
