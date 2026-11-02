//! With every variant under `#[cfg]`, no variant can supply an associated
//! item in every build; the enum has to fix it.
#[whichever::dispatchable]
trait Marker {
    type Tag;
    fn tag(&self) -> u8;
}

struct Present;

impl Marker for Present {
    type Tag = u8;
    fn tag(&self) -> u8 {
        1
    }
}

#[whichever::dispatch(Marker)]
enum Any {
    #[cfg(all())]
    Present(Present),
}

fn main() {}
