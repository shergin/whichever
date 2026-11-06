//! `into = Target` converts every variant's payload, so a variant without
//! one is reported.
pub struct Text(pub String);
pub struct Printed(pub String);

impl From<Text> for Printed {
    fn from(text: Text) -> Self {
        Printed(text.0)
    }
}

#[whichever::dispatch(into = Printed)]
pub enum Value {
    Text(Text),
    Nothing,
}

fn main() {}
