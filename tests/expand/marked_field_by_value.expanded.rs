//! The marked field of a variant with several, beside a routed tuple
//! variant: where the return type asks for a value the field is read
//! through the binding, `*inner`, to match the routed `inner.handle`; where
//! it asks for a reference the binding is that reference. Under `with =
//! Some` the return type's first argument asks, and any other lift is
//! handed the binding as it stands.
pub struct Dictionary {
    pub handle: u64,
}
pub enum Object {
    Dictionary(Dictionary),
    Tagged(u64, u32),
    Unknown { name: String, handle: u64 },
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Object_7bc0879a47b651b0_11_1 as Object;
fn raw(handle: &u64) -> u64 {
    *handle
}
impl Object {
    pub fn handle(&self) -> u64 {
        match self {
            Self::Dictionary(__inner) => __inner.handle,
            Self::Tagged(__inner, ..) => *__inner,
            Self::Unknown { handle: __inner, .. } => *__inner,
        }
    }
    pub fn handle_ref(&self) -> &u64 {
        match self {
            Self::Dictionary(__inner) => &__inner.handle,
            Self::Tagged(__inner, ..) => __inner,
            Self::Unknown { handle: __inner, .. } => __inner,
        }
    }
    pub fn handle_mut(&mut self) -> &mut u64 {
        match self {
            Self::Dictionary(__inner) => &mut __inner.handle,
            Self::Tagged(__inner, ..) => __inner,
            Self::Unknown { handle: __inner, .. } => __inner,
        }
    }
    pub fn some_handle(&self) -> Option<u64> {
        match self {
            Self::Dictionary(__inner) => Some(__inner.handle),
            Self::Tagged(__inner, ..) => Some(*__inner),
            Self::Unknown { handle: __inner, .. } => Some(*__inner),
        }
    }
    pub fn some_handle_ref(&self) -> Option<&u64> {
        match self {
            Self::Dictionary(__inner) => Some(&__inner.handle),
            Self::Tagged(__inner, ..) => Some(__inner),
            Self::Unknown { handle: __inner, .. } => Some(__inner),
        }
    }
    pub fn raw_of_marked(&self) -> u64 {
        match self {
            Self::Tagged(__inner, ..) => raw(__inner),
            Self::Unknown { handle: __inner, .. } => raw(__inner),
            _ => 0,
        }
    }
}
fn main() {}
