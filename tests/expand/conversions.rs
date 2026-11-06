//! The by-reference `TryFrom` forms, and `into = Target` through every
//! payload's own `Into`.
pub struct Text(pub String);
pub struct Number(pub u32);
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

#[whichever::dispatch(try_from = ref_mut, into = Printed)]
pub enum Value {
    Text(Text),
    Number(Number),
}

fn main() {}
