#[whichever::dispatchable]
trait Tagged {
    const TAG: u8;
    fn kinds() -> &'static [&'static str] {
        &["default"]
    }
    fn value(&self) -> u8;
}

struct Left;
struct Right;

impl Tagged for Left {
    const TAG: u8 = 7;
    fn kinds() -> &'static [&'static str] {
        &["left"]
    }
    fn value(&self) -> u8 {
        1
    }
}

impl Tagged for Right {
    const TAG: u8 = 7;
    fn value(&self) -> u8 {
        2
    }
}

#[whichever::dispatch(Tagged)]
enum Side {
    Left(Left),
    Right(Right),
}

#[test]
fn consts_come_from_the_first_variant_and_statics_keep_their_default() {
    assert_eq!(<Side as Tagged>::TAG, 7);
    assert_eq!(<Side as Tagged>::kinds(), &["default"]);
    assert_eq!(Side::from(Right).value(), 2);
}
