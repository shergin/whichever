//! A struct that marks no field is one error, and its impl is still
//! written, so the coercion below finds it.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
}

#[whichever::dispatch]
pub struct Labelled {
    inner: Box<dyn Shape>,
    pub label: u8,
}

#[whichever::dispatch]
impl Shape for Labelled {
    fn name(&self) -> &'static str {
        "labelled"
    }
}

pub fn boxed(inner: Box<dyn Shape>) -> Box<dyn Shape> {
    Box::new(Labelled { inner, label: 0 })
}

fn main() {}
