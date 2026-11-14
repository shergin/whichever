//! The marked field of a variant with several, beside a routed tuple
//! variant: where the return type asks for a value the field is read
//! through the binding, `*inner`, to match the routed `inner.handle`; where
//! it asks for a reference the binding is that reference. Under `with =
//! Some` the return type's first argument asks, and any other lift is
//! handed the binding as it stands.
pub struct Handle(pub u64);
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for Handle {}
#[automatically_derived]
impl ::core::clone::Clone for Handle {
    #[inline]
    fn clone(&self) -> Self {
        let _: ::core::clone::AssertParamIsClone<u64>;
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for Handle {}
pub struct Dictionary {
    pub handle: Handle,
}
pub enum Object {
    Dictionary(Dictionary),
    Tagged(Handle, u32),
    Unknown { name: String, handle: Handle },
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Object_846bc8e290b80da4_14_1 as Object;
fn raw(handle: &Handle) -> u64 {
    handle.0
}
impl Object {
    pub fn handle(&self) -> Handle {
        match self {
            Self::Dictionary(__inner) => __inner.handle,
            Self::Tagged(__inner, ..) => *__inner,
            Self::Unknown { handle: __inner, .. } => *__inner,
        }
    }
    pub fn handle_ref(&self) -> &Handle {
        match self {
            Self::Dictionary(__inner) => &__inner.handle,
            Self::Tagged(__inner, ..) => __inner,
            Self::Unknown { handle: __inner, .. } => __inner,
        }
    }
    pub fn handle_mut(&mut self) -> &mut Handle {
        match self {
            Self::Dictionary(__inner) => &mut __inner.handle,
            Self::Tagged(__inner, ..) => __inner,
            Self::Unknown { handle: __inner, .. } => __inner,
        }
    }
    pub fn some_handle(&self) -> Option<Handle> {
        match self {
            Self::Dictionary(__inner) => Some(__inner.handle),
            Self::Tagged(__inner, ..) => Some(*__inner),
            Self::Unknown { handle: __inner, .. } => Some(*__inner),
        }
    }
    pub fn some_handle_ref(&self) -> Option<&Handle> {
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
