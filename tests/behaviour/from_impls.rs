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
