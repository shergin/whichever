//! An associated type or const a partial block declares under `#[cfg]` is
//! reported: the forwarded methods convert into the type in every build,
//! so the whole block goes under the `cfg`, with a second block for the
//! other builds.
#[whichever::dispatchable]
trait Handler {
    type Value;
    const LIMIT: u32;
    fn value(&self) -> Self::Value;
}

struct Text;

impl Handler for Text {
    type Value = u32;
    const LIMIT: u32 = 1;
    fn value(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
enum AnyHandler {
    Text(Text),
}

#[whichever::dispatch]
impl Handler for AnyHandler {
    #[cfg(feature = "wide")]
    type Value = u64;

    #[cfg(feature = "wide")]
    const LIMIT: u32 = 2;
}

fn main() {}
