//! A method a partial block writes under `#[cfg]`: the block's item stays
//! under its `cfg`, and the forwarded method is written under the
//! complement, as a hand writes the two arms. `cargo expand` evaluates the
//! `cfg`, so this golden shows the build where the feature is off: the
//! forwarded `name` stands and the block's is gone.
#[whichever::dispatchable]
pub trait Tree {
    fn get(&self) -> u32;
    fn name(&self) -> &'static str {
        "default"
    }
}

pub struct Inner;

impl Tree for Inner {
    fn get(&self) -> u32 {
        1
    }
    fn name(&self) -> &'static str {
        "inner"
    }
}

#[whichever::dispatch]
pub enum AnyTree {
    Inner(Inner),
}

#[whichever::dispatch]
impl Tree for AnyTree {
    #[cfg(feature = "named")]
    #[cfg(not(test))]
    fn name(&self) -> &'static str {
        "enum"
    }
}

fn main() {}
