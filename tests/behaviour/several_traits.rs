#[whichever::dispatchable]
trait Base {
    fn base(&self) -> u8;
}

#[whichever::dispatchable]
trait Derived: Base {
    fn derived(&self) -> u8 {
        self.base() * 10
    }
}

struct One;
struct Two;

impl Base for One {
    fn base(&self) -> u8 {
        1
    }
}

impl Base for Two {
    fn base(&self) -> u8 {
        2
    }
}

impl Derived for One {}

impl Derived for Two {
    fn derived(&self) -> u8 {
        99
    }
}

#[whichever::dispatch(Base)]
#[dispatch(Derived)]
enum Number {
    One(One),
    Two(Two),
}

#[test]
fn each_attribute_dispatches_one_trait() {
    assert_eq!(Number::from(One).derived(), 10);
    assert_eq!(Number::from(Two).derived(), 99);
    assert_eq!(Number::from(Two).base(), 2);
}
