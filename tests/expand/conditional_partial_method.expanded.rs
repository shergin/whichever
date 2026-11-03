//! A method a partial block writes under `#[cfg]`: the block's item stays
//! under its `cfg`, and the forwarded method is written under the
//! complement, as a hand writes the two arms. `cargo expand` evaluates the
//! `cfg`, so this golden shows the build where the feature is off: the
//! forwarded `name` stands and the block's is gone.
pub trait Tree {
    fn get(&self) -> u32;
    fn name(&self) -> &'static str {
        "default"
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Tree_f3eec59df3d4c121_6_1 as Tree;
pub struct Inner;
impl Tree for Inner {
    fn get(&self) -> u32 {
        1
    }
    fn name(&self) -> &'static str {
        "inner"
    }
}
pub enum AnyTree {
    Inner(Inner),
}
#[automatically_derived]
impl ::core::convert::From<Inner> for AnyTree {
    #[inline]
    fn from(__value: Inner) -> Self {
        Self::Inner(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_AnyTree_ed84ca8ceef98a85_25_1 as AnyTree;
impl Tree for AnyTree {
    #[inline]
    fn get(&self) -> u32 {
        match self {
            AnyTree::Inner(__inner) => <Inner as Tree>::get(__inner),
        }
    }
    #[inline]
    fn name(&self) -> &'static str {
        match self {
            AnyTree::Inner(__inner) => <Inner as Tree>::name(__inner),
        }
    }
}
fn main() {}
