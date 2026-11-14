//! A plain payload beside a routed tuple variant, and a struct's marked
//! field beside another: every target the pattern binds is read through
//! the binding, `*inner`, where the return type asks for a value, and the
//! binding is the arm where it asks for a reference. Under `with = Some`
//! the return type's first argument asks; any other lift, a raw pointer
//! and an opaque type are handed the binding as it stands.
pub struct Dictionary {
    pub handle: u64,
}
pub enum Object {
    Dictionary(Dictionary),
    Plain(u64),
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Object_638aaf163710e033_11_1 as Object;
fn raw(handle: &u64) -> u64 {
    *handle
}
impl Object {
    pub fn handle(&self) -> u64 {
        match self {
            Self::Dictionary(__inner) => __inner.handle,
            Self::Plain(__inner) => *__inner,
        }
    }
    pub fn handle_ref(&self) -> &u64 {
        match self {
            Self::Dictionary(__inner) => &__inner.handle,
            Self::Plain(__inner) => __inner,
        }
    }
    pub fn handle_mut(&mut self) -> &mut u64 {
        match self {
            Self::Dictionary(__inner) => &mut __inner.handle,
            Self::Plain(__inner) => __inner,
        }
    }
    pub fn some_handle(&self) -> Option<u64> {
        match self {
            Self::Dictionary(__inner) => Some(__inner.handle),
            Self::Plain(__inner) => Some(*__inner),
        }
    }
    pub fn raw_of_plain(&self) -> u64 {
        match self {
            Self::Plain(__inner) => raw(__inner),
            _ => 0,
        }
    }
    pub fn handle_ptr(&self) -> *const u64 {
        match self {
            Self::Plain(__inner) => __inner,
            _ => core::ptr::null(),
        }
    }
}
pub struct Slot {
    handle: u64,
    label: String,
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Slot_4f5ad6d127b93b43_47_1 as Slot;
impl Slot {
    pub fn handle(&self) -> u64 {
        match self {
            Self { handle: __inner, .. } => *__inner,
        }
    }
    pub fn handle_ref(&self) -> &u64 {
        match self {
            Self { handle: __inner, .. } => __inner,
        }
    }
    pub fn shown(&self) -> impl core::fmt::Display + '_ {
        match self {
            Self { label: __inner, .. } => __inner,
        }
    }
}
fn main() {}
