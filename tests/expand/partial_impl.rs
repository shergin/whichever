#[whichever::dispatchable]
pub trait Handler {
    type Value;
    fn make(seed: u32) -> Option<Self>
    where
        Self: Sized;
    fn value(&self) -> Self::Value;
    fn describe(&self) -> String {
        String::from("handler")
    }
}

pub struct Text;
pub struct Map;

impl Handler for Text {
    type Value = u32;
    fn make(_seed: u32) -> Option<Self> {
        Some(Text)
    }
    fn value(&self) -> u32 {
        1
    }
}

impl Handler for Map {
    type Value = u8;
    fn make(_seed: u32) -> Option<Self> {
        None
    }
    fn value(&self) -> u8 {
        2
    }
}

#[whichever::dispatch]
pub enum AnyHandler {
    Text(Text),
    Map(Map),
}

#[whichever::dispatch]
impl Handler for AnyHandler {
    /// The enum's own value type; each payload's converts into it.
    type Value = u64;

    fn make(seed: u32) -> Option<Self> {
        Text::make(seed).map(AnyHandler::Text)
    }

    fn describe(&self) -> String {
        String::from("any handler")
    }
}

fn main() {}
