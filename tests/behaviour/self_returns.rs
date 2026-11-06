#[whichever::dispatchable]
trait Grow {
    fn grow(&self) -> Self
    where
        Self: Sized;
    fn halve(&self) -> Option<Self>
    where
        Self: Sized;
    fn checked(&self) -> Result<Self, &'static str>
    where
        Self: Sized;
}

#[derive(Debug, PartialEq)]
struct Small(u8);
#[derive(Debug, PartialEq)]
struct Big(u32);

impl Grow for Small {
    fn grow(&self) -> Self {
        Small(self.0 + 1)
    }
    fn halve(&self) -> Option<Self> {
        self.0.is_multiple_of(2).then_some(Small(self.0 / 2))
    }
    fn checked(&self) -> Result<Self, &'static str> {
        Ok(Small(self.0))
    }
}

impl Grow for Big {
    fn grow(&self) -> Self {
        Big(self.0 + 1)
    }
    fn halve(&self) -> Option<Self> {
        Some(Big(self.0 / 2))
    }
    fn checked(&self) -> Result<Self, &'static str> {
        Err("too big")
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(Grow)]
enum Number {
    Small(Small),
    Big(Big),
}

#[test]
fn results_are_wrapped_back_into_the_same_variant() {
    let small: Number = Small(4).into();
    let big: Number = Big(4).into();
    assert_eq!(small.grow(), Number::Small(Small(5)));
    assert_eq!(big.grow(), Number::Big(Big(5)));
    assert_eq!(small.halve(), Some(Number::Small(Small(2))));
    assert_eq!(Number::from(Small(3)).halve(), None);
    assert_eq!(small.checked(), Ok(Number::Small(Small(4))));
    assert_eq!(big.checked(), Err("too big"));
}
