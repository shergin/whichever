#[whichever::dispatchable]
pub trait Describe {
    fn name(&self) -> &'static str;
    fn kind(&self) -> &'static str {
        "default"
    }
}

pub struct Cat;
pub struct Dog;

impl Describe for Cat {
    fn name(&self) -> &'static str {
        "cat"
    }
}

impl Describe for Dog {
    fn name(&self) -> &'static str {
        "dog"
    }
}

#[whichever::dispatch(Describe)]
pub enum Animal {
    Cat(Cat),
    Dog(Dog),
}

fn main() {}
