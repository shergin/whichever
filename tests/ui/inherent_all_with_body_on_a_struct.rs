//! A struct has one field and no variant for an arm to name, so under
//! `#[dispatch(*)]` a body would never run, whatever it holds.
pub struct Circle;

impl Circle {
    pub fn radius(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
pub struct Wrapper(Circle);

#[whichever::dispatch]
impl Wrapper {
    #[dispatch(*)]
    pub fn radius(&self) -> u32 {
        match self {
            Self(_) => 0,
        }
    }
}

fn main() {}
