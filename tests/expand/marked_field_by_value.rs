//! The marked field of a variant with several, beside a routed tuple
//! variant: where the return type asks for a value the field is read
//! through the binding, `*inner`, to match the routed `inner.handle`; where
//! it asks for a reference the binding is that reference. Under `with =
//! Some` the return type's first argument asks, and any other lift is
//! handed the binding as it stands.
pub struct Dictionary {
    pub handle: u64,
}

#[whichever::dispatch(from = false)]
pub enum Object {
    #[dispatch(via = .handle)]
    Dictionary(Dictionary),
    Tagged(#[dispatch] u64, u32),
    Unknown {
        name: String,
        #[dispatch]
        handle: u64,
    },
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

    #[dispatch(*, value, with = Some)]
    pub fn some_handle_ref(&self) -> Option<&u64>;

    #[dispatch(Tagged, Unknown, value, with = raw)]
    pub fn raw_of_marked(&self) -> u64 {
        0
    }
}

fn main() {}
