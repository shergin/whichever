//! An inherent method may name a variant under `#[cfg]`; where the variant
//! is compiled out, the method takes its body.

pub struct Plain;

impl Plain {
    pub fn root(&self) -> u32 {
        1
    }
}

#[cfg(any())]
pub struct Proving;

#[whichever::dispatch]
pub enum Header {
    Plain(Plain),
    #[cfg(any())]
    Proving(Proving),
}

#[whichever::dispatch]
impl Header {
    #[dispatch(Proving, with = Some)]
    pub fn registry_root(&self) -> Option<u32> {
        None
    }

    // Naming every variant, the conditional one included, is exhaustive
    // in every build, so the method takes no body.
    #[dispatch(Plain, Proving)]
    pub fn root(&self) -> u32;
}

#[test]
fn a_variant_compiled_out_takes_the_body() {
    let header = Header::Plain(Plain);
    assert_eq!(header.registry_root(), None);
    assert_eq!(header.root(), 1);
}
