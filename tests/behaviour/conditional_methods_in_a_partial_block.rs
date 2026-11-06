//! A method a partial block writes under `#[cfg]` is the block's where the
//! `cfg` holds and forwarded where it does not, so the payload's override is
//! never hidden by the trait's default.

#[whichever::dispatchable]
pub trait Tree {
    fn get(&self) -> u32;
    fn name(&self) -> &'static str {
        "default"
    }
    fn kind(&self) -> &'static str {
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
    fn kind(&self) -> &'static str {
        "inner"
    }
}

pub struct Other;

impl Tree for Other {
    fn get(&self) -> u32 {
        2
    }
}

#[whichever::dispatch]
pub enum AnyTree {
    Inner(Inner),
    Other(Other),
}

// An integration test is compiled with `cfg(test)` set, so `not(test)`
// is the arm that is off and `test` the one that is on.
#[whichever::dispatch]
impl Tree for AnyTree {
    #[cfg(not(test))]
    fn name(&self) -> &'static str {
        "enum"
    }

    #[cfg(test)]
    fn kind(&self) -> &'static str {
        "enum"
    }
}

#[whichever::dispatch]
pub struct Wrapped {
    #[dispatch]
    inner: Inner,
    pub label: u8,
}

#[whichever::dispatch]
impl Tree for Wrapped {
    #[cfg(not(test))]
    fn name(&self) -> &'static str {
        "wrapped"
    }
}

#[test]
fn the_payload_answers_where_the_cfg_is_off() {
    assert_eq!(AnyTree::Inner(Inner).name(), "inner");
    assert_eq!(AnyTree::Other(Other).name(), "default");
    assert_eq!(AnyTree::Inner(Inner).kind(), "enum");
    assert_eq!(AnyTree::Inner(Inner).get(), 1);
    let wrapped = Wrapped {
        inner: Inner,
        label: 1,
    };
    assert_eq!(
        (wrapped.name(), wrapped.get(), wrapped.label),
        ("inner", 1, 1)
    );
}
