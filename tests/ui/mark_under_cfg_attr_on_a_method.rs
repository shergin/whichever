//! A mark under `cfg_attr` on a method of an inherent block or of a partial
//! impl block is read in no build; it is one sentence naming the way
//! around, and the method is read as if the mark were plain.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

impl Square {
    pub fn describe(&self) -> String {
        String::from("square")
    }
}

#[whichever::dispatch]
pub enum Any {
    Square(Square),
    Empty,
}

#[whichever::dispatch]
impl Any {
    #[cfg_attr(all(), dispatch(Square))]
    pub fn describe(&self) -> String {
        String::from("nothing")
    }
}

#[whichever::dispatch]
pub enum Only {
    Square(Square),
}

#[whichever::dispatch]
impl Only {
    #[cfg_attr(all(), dispatch(*))]
    pub fn describe(&self) -> String;
}

#[whichever::dispatch]
impl Shape for Only {
    #[cfg_attr(all(), dispatch(default))]
    fn name(&self) -> &'static str;
}

fn main() {}
