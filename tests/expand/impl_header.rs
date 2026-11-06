//! An empty partial impl block gives the impl a header the enum's attribute
//! cannot write: here, the trait's own parameter, for every `F`.
pub trait Format {
    type Value;
}

#[whichever::dispatchable]
pub trait Encoder<F: Format> {
    fn encode(&self, value: &F::Value) -> usize;
}

pub struct Json;
impl Format for Json {
    type Value = String;
}

pub struct A;
impl<F: Format> Encoder<F> for A {
    fn encode(&self, _: &F::Value) -> usize {
        1
    }
}

#[whichever::dispatch]
pub enum AnyEncoder {
    A(A),
}

#[whichever::dispatch]
impl<F: Format> Encoder<F> for AnyEncoder {}

fn main() {}
