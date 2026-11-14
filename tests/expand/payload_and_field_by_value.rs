//! A plain payload beside a routed tuple variant, and a struct's marked
//! field beside another: every target the pattern binds is read through
//! the binding, `*inner`, where the return type asks for a value, and the
//! binding is the arm where it asks for a reference. Under `with = Some`
//! the return type's first argument asks; any other lift, a raw pointer
//! and an opaque type are handed the binding as it stands.
pub struct Dictionary {
    pub handle: u64,
}

#[whichever::dispatch(from = false)]
pub enum Object {
    #[dispatch(via = .handle)]
    Dictionary(Dictionary),
    Plain(u64),
}

fn raw(handle: &u64) -> u64 {
    *handle
}

#[whichever::dispatch]
impl Object {
    #[dispatch(*, value)]
    pub fn handle(&self) -> u64;

    #[dispatch(*, value)]
    pub fn handle_ref(&self) -> &u64;

    #[dispatch(*, value)]
    pub fn handle_mut(&mut self) -> &mut u64;

    #[dispatch(*, value, with = Some)]
    pub fn some_handle(&self) -> Option<u64>;

    #[dispatch(Plain, value, with = raw)]
    pub fn raw_of_plain(&self) -> u64 {
        0
    }

    #[dispatch(Plain, value)]
    pub fn handle_ptr(&self) -> *const u64 {
        core::ptr::null()
    }
}

#[whichever::dispatch]
pub struct Slot {
    #[dispatch]
    handle: u64,
    label: String,
}

#[whichever::dispatch]
impl Slot {
    #[dispatch(*, value)]
    pub fn handle(&self) -> u64;

    #[dispatch(*, value)]
    pub fn handle_ref(&self) -> &u64;

    #[dispatch(label, value)]
    pub fn shown(&self) -> impl core::fmt::Display + '_;
}

fn main() {}
