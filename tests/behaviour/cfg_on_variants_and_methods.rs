#[whichever::dispatchable]
trait Gated {
    fn always(&self) -> u8;
    #[cfg(any())]
    fn never(&self) -> Missing;
}

struct Present;

impl Gated for Present {
    fn always(&self) -> u8 {
        1
    }
}

#[whichever::dispatch(Gated)]
enum Gate {
    Present(Present),
    #[cfg(any())]
    Absent(Missing),
}

#[test]
fn disabled_items_are_disabled_everywhere() {
    assert_eq!(Gate::from(Present).always(), 1);
}

/// A variant the method cannot be forwarded to, a `deref` one beside a
/// method returning `Self`, under a `cfg` that does not hold: its arm, and
/// the sentence the arm would carry, go with it, as a hand's arm does.
mod refused_variant_compiled_out {
    #[whichever::dispatchable]
    pub trait Grow {
        fn grown(&self) -> Self;
        fn side(&self) -> u64;
    }

    #[derive(Debug, PartialEq)]
    pub struct Square(pub u64);

    impl Grow for Square {
        fn grown(&self) -> Self {
            Square(self.0 + 1)
        }
        fn side(&self) -> u64 {
            self.0
        }
    }

    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Grow)]
    pub enum Shape {
        Plain(Square),
        #[cfg(any())]
        #[dispatch(deref)]
        Boxed(Box<Square>),
    }

    #[test]
    fn the_variant_present_is_forwarded() {
        let grown = Shape::Plain(Square(2)).grown();
        assert_eq!(grown, Shape::Plain(Square(3)));
        assert_eq!(grown.side(), 3);
    }
}

/// A method of a `#[dispatch]` impl block that cannot be forwarded, under a
/// `cfg` that does not hold, is not in the build, as a hand's method under
/// that `cfg` is not, and its sentence goes with it: one without a receiver,
/// one whose mark does not parse, and one without a body or a mark. The
/// method beside them is forwarded.
mod inherent_methods_compiled_out {
    pub struct Circle(pub u32);

    impl Circle {
        pub fn area(&self) -> u32 {
            3 * self.0 * self.0
        }
    }

    pub struct Square(pub u32);

    impl Square {
        pub fn area(&self) -> u32 {
            self.0 * self.0
        }
    }

    #[whichever::dispatch]
    pub enum Shape {
        Circle(Circle),
        Square(Square),
    }

    #[whichever::dispatch]
    impl Shape {
        #[cfg(any())]
        #[dispatch(*)]
        pub fn make(side: u32) -> Self;

        #[cfg(any())]
        #[dispatch(*, Circle)]
        pub fn doubled(&self) -> u32;

        #[cfg(any())]
        pub fn unmarked(&self) -> u32;

        #[cfg(any())]
        #[dispatch(*)]
        #[dispatch(Circle)]
        pub fn marked_twice(&self) -> u32;

        #[cfg(any())]
        #[dispatch(Triangle)]
        pub fn lacking(&self) -> u32 {
            0
        }

        #[dispatch(*)]
        pub fn area(&self) -> u32;
    }

    #[test]
    fn the_enum_forwards_the_method_in_the_build() {
        assert_eq!(Shape::Circle(Circle(1)).area(), 3);
        assert_eq!(Shape::Square(Square(2)).area(), 4);
    }

    #[whichever::dispatch]
    pub struct Labelled {
        #[dispatch]
        shape: Square,
        pub label: &'static str,
    }

    #[whichever::dispatch]
    impl Labelled {
        #[cfg(any())]
        #[dispatch(*)]
        pub fn make(side: u32) -> Self;

        #[cfg(any())]
        #[dispatch(*, shape)]
        pub fn doubled(&self) -> u32;

        #[cfg(any())]
        pub fn unmarked(&self) -> u32;

        #[dispatch(*)]
        pub fn area(&self) -> u32;
    }

    #[test]
    fn the_struct_forwards_the_method_in_the_build() {
        let labelled = Labelled {
            shape: Square(3),
            label: "a",
        };
        assert_eq!(labelled.area(), 9);
    }
}
