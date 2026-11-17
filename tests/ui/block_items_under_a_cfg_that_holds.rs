//! An item of a partial impl block that cannot be read is reported under
//! its own `#[cfg]`: where the `cfg` holds, its sentence stands as it
//! would without one, and each such item has its own.
#[whichever::dispatchable]
trait Labels {
    fn get(&self) -> u32;
    fn bodiless(&self) -> u32 {
        0
    }
    fn unparsed(&self) -> u32 {
        0
    }
    fn inherent_option(&self) -> u32 {
        0
    }
    fn marked_twice(&self) -> u32 {
        0
    }
    fn default_with_a_body(&self) -> u32 {
        0
    }
}

struct Inner;

impl Labels for Inner {
    fn get(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
enum Any {
    Inner(Inner),
}

#[whichever::dispatch]
impl Labels for Any {
    #[cfg(all())]
    fn bodiless(&self) -> u32;

    #[cfg(all())]
    #[dispatch(=)]
    fn unparsed(&self) -> u32;

    #[cfg(all())]
    #[dispatch(*, call = other)]
    fn inherent_option(&self) -> u32;

    #[cfg(all())]
    #[dispatch(default)]
    #[dispatch(*)]
    fn marked_twice(&self) -> u32;

    #[cfg(all())]
    #[dispatch(default)]
    fn default_with_a_body(&self) -> u32 {
        2
    }
}

fn main() {}
