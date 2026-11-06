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
