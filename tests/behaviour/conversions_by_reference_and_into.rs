//! `try_from = ref` and `ref_mut` add the by-reference `TryFrom` impls, and
//! `into = Target` converts the enum into a type every payload converts into.

#[derive(Debug, PartialEq)]
pub struct Text(pub String);
#[derive(Debug, PartialEq)]
pub struct Number(pub u32);

#[derive(Debug, PartialEq)]
pub struct Printed(pub String);

impl From<Text> for Printed {
    fn from(text: Text) -> Self {
        Printed(text.0)
    }
}

impl From<Number> for Printed {
    fn from(number: Number) -> Self {
        Printed(number.0.to_string())
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(try_from = ref_mut, into = Printed)]
pub enum Value {
    Text(Text),
    Number(Number),
}

#[test]
fn the_payload_is_reached_by_value_by_reference_and_mutably() {
    let mut value = Value::Number(Number(1));
    assert_eq!(<&Number>::try_from(&value), Ok(&Number(1)));
    assert_eq!(<&Text>::try_from(&value), Err(&Value::Number(Number(1))));
    if let Ok(number) = <&mut Number>::try_from(&mut value) {
        number.0 = 2;
    }
    assert_eq!(Number::try_from(value), Ok(Number(2)));
    let printed: Printed = Value::Text(Text("x".into())).into();
    assert_eq!(printed, Printed("x".into()));
    let printed: Printed = Value::Number(Number(3)).into();
    assert_eq!(printed, Printed("3".into()));
}
