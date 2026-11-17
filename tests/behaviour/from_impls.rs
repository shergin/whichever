#[whichever::dispatchable]
trait Value {
    fn value(&self) -> u8;
}

struct Left(u8);

impl Value for Left {
    fn value(&self) -> u8 {
        self.0
    }
}

impl Value for u8 {
    fn value(&self) -> u8 {
        *self
    }
}

#[whichever::dispatch(Value, from = false)]
enum Handmade {
    Left(Left),
}

impl From<Left> for Handmade {
    fn from(left: Left) -> Self {
        Handmade::Left(Left(left.0 + 100))
    }
}

#[whichever::dispatch(Value)]
enum Twice {
    First(u8),
    Second(u8),
}

impl From<u8> for Twice {
    fn from(value: u8) -> Self {
        Twice::Second(value)
    }
}

#[test]
fn generation_can_be_switched_off_and_skips_repeated_payloads() {
    assert_eq!(Handmade::from(Left(1)).value(), 101);
    assert!(matches!(Twice::from(3), Twice::Second(3)));
}

/// A projection from a type that is not a type parameter of the enum is a
/// type rustc can name, so it gets its `From` and its `TryFrom`; one from a
/// type parameter is left out with `from = false` on its variant.
mod projections {
    use super::{Left, Value};

    trait Source {
        type Out: Value;
    }

    struct Fixed;

    impl Source for Fixed {
        type Out = Left;
    }

    #[whichever::dispatch(Value, try_from = true)]
    enum Concrete {
        Out(<Fixed as Source>::Out),
        Byte(u8),
    }

    #[whichever::dispatch(Value)]
    enum Generic<S: Source> {
        #[dispatch(from = false)]
        Out(S::Out),
        Byte(u8),
    }

    #[test]
    fn a_projection_from_a_concrete_type_converts() {
        let concrete = Concrete::from(Left(4));
        assert_eq!(concrete.value(), 4);
        let back = Left::try_from(concrete).ok().unwrap();
        assert_eq!(back.0, 4);
        assert!(Left::try_from(Concrete::from(9u8)).is_err());
    }

    #[test]
    fn a_projection_from_a_parameter_is_left_out_and_still_dispatched() {
        let generic: Generic<Fixed> = Generic::Out(Left(6));
        assert_eq!(generic.value(), 6);
        assert_eq!(Generic::<Fixed>::from(2u8).value(), 2);
    }
}

/// `From` reaches one level: an enum held by another gets one blanket impl
/// by hand, through the inner enum's own `From`, with the variant's own
/// left out by `from = false`, and each level reaches the next through its
/// own.
mod nested {
    #[whichever::dispatchable]
    pub trait Shape {
        fn area(&self) -> u32;
    }

    pub struct Circle(pub u32);
    pub struct Square(pub u32);
    pub struct Line(pub u32);

    impl Shape for Circle {
        fn area(&self) -> u32 {
            3 * self.0 * self.0
        }
    }

    impl Shape for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
    }

    impl Shape for Line {
        fn area(&self) -> u32 {
            0
        }
    }

    #[whichever::dispatch(Shape)]
    pub enum Round {
        Circle(Circle),
    }

    #[whichever::dispatch(Shape)]
    pub enum Flat {
        Square(Square),
        #[dispatch(from = false)]
        Round(Round),
    }

    impl<T> From<T> for Flat
    where
        Round: From<T>,
    {
        fn from(value: T) -> Self {
            Flat::Round(Round::from(value))
        }
    }

    #[whichever::dispatch(Shape)]
    pub enum AnyShape {
        #[dispatch(from = false)]
        Flat(Flat),
        Line(Line),
    }

    impl<T> From<T> for AnyShape
    where
        Flat: From<T>,
    {
        fn from(value: T) -> Self {
            AnyShape::Flat(Flat::from(value))
        }
    }

    #[test]
    fn every_leaf_converts_through_two_levels() {
        assert!(matches!(
            Flat::from(Circle(1)),
            Flat::Round(Round::Circle(Circle(1)))
        ));
        assert!(matches!(Flat::from(Square(2)), Flat::Square(Square(2))));
        let round = Round::from(Circle(3));
        assert!(matches!(Flat::from(round), Flat::Round(Round::Circle(_))));
        assert_eq!(Flat::from(Square(2)).area(), 4);
    }

    #[test]
    fn every_leaf_converts_through_three_levels() {
        assert!(matches!(
            AnyShape::from(Circle(1)),
            AnyShape::Flat(Flat::Round(Round::Circle(Circle(1))))
        ));
        let square: AnyShape = Square(2).into();
        assert!(matches!(square, AnyShape::Flat(Flat::Square(Square(2)))));
        assert!(matches!(
            AnyShape::from(Round::from(Circle(4))),
            AnyShape::Flat(Flat::Round(_))
        ));
        assert!(matches!(
            AnyShape::from(Flat::from(Square(5))),
            AnyShape::Flat(Flat::Square(_))
        ));
        assert!(matches!(AnyShape::from(Line(6)), AnyShape::Line(Line(6))));
        assert_eq!(AnyShape::from(Circle(1)).area(), 3);
    }
}
