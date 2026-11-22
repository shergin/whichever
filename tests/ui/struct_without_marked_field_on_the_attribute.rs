//! A struct that marks no field for the trait its attribute names is one
//! error, and its impl is still written, so the coercion below finds it.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
}

#[whichever::dispatch(Shape)]
pub struct Labelled {
    inner: Box<dyn Shape>,
    pub label: u8,
}

pub fn boxed(inner: Box<dyn Shape>) -> Box<dyn Shape> {
    Box::new(Labelled { inner, label: 0 })
}

fn main() {}
