//! With every variant under `#[cfg]`, no variant can supply an associated
//! type or const in every build. Each is written holding its sentence, so
//! the impl is complete: the calls through the trait resolve, a method
//! returning the type is forwarded, and the sentences stand alone.
#[whichever::dispatchable]
trait Marker {
    type Tag;
    const LEVEL: u8;
    fn tag(&self) -> u8;
    fn tagged(&self) -> Self::Tag;
}

struct Present;

impl Marker for Present {
    type Tag = u8;
    const LEVEL: u8 = 1;
    fn tag(&self) -> u8 {
        1
    }
    fn tagged(&self) -> u8 {
        1
    }
}

#[whichever::dispatch(Marker)]
enum Any {
    #[cfg(all())]
    Present(Present),
}

fn call(any: &Any) -> u8 {
    let tag: u8 = any.tagged();
    tag + any.tag() + <Any as Marker>::LEVEL
}

fn main() {
    let _ = call(&Any::Present(Present));
}
