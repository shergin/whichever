//! The marked field of a variant with several, beside a routed tuple
//! variant: where the return type asks for a value the field is read
//! through the binding, `*inner`, to match the routed `inner.handle`; where
//! it asks for a reference the binding is that reference. Under `with =
//! Some` the return type's first argument asks, and any other lift is
//! handed the binding as it stands.
#[derive(Clone, Copy)]
pub struct Handle(pub u64);

pub struct Dictionary {
    pub handle: Handle,
}

#[whichever::dispatch(from = false)]
pub enum Object {
    #[dispatch(via = .handle)]
    Dictionary(Dictionary),
    Tagged(#[dispatch] Handle, u32),
    Unknown {
        name: String,
        #[dispatch]
        handle: Handle,
    },
}

fn raw(handle: &Handle) -> u64 {
    handle.0
}

#[whichever::dispatch]
impl Object {
    #[dispatch(*, value)]
    pub fn handle(&self) -> Handle;

    #[dispatch(*, value)]
    pub fn handle_ref(&self) -> &Handle;

    #[dispatch(*, value)]
    pub fn handle_mut(&mut self) -> &mut Handle;

    #[dispatch(*, value, with = Some)]
    pub fn some_handle(&self) -> Option<Handle>;

    #[dispatch(*, value, with = Some)]
    pub fn some_handle_ref(&self) -> Option<&Handle>;

    #[dispatch(Tagged, Unknown, value, with = raw)]
    pub fn raw_of_marked(&self) -> u64 {
        0
    }
}

fn main() {}
