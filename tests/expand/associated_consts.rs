#[whichever::dispatchable]
pub trait Proof {
    const KIND: &'static str;
    const ARITY: usize;
    fn check(&self) -> bool;
}

pub struct Encoding;
pub struct Ordering;

impl Proof for Encoding {
    const KIND: &'static str = "encoding";
    const ARITY: usize = 1;
    fn check(&self) -> bool {
        true
    }
}

impl Proof for Ordering {
    const KIND: &'static str = "ordering";
    const ARITY: usize = 1;
    fn check(&self) -> bool {
        false
    }
}

#[whichever::dispatch(Proof, KIND = "any")]
pub enum AnyProof {
    Encoding(Encoding),
    Ordering(Ordering),
}

fn main() {}
