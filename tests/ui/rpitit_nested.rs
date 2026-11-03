//! An opaque type is found at any depth of an enum's return type, not only
//! at the top, so rustc's "match arms have incompatible types" never shows.
#[whichever::dispatchable]
trait Source {
    fn items(&self) -> Result<impl Iterator<Item = u32> + '_, ()>;
}

struct Few(Vec<u32>);
struct One(Option<u32>);

impl Source for Few {
    fn items(&self) -> Result<impl Iterator<Item = u32> + '_, ()> {
        Ok(self.0.iter().copied())
    }
}

impl Source for One {
    fn items(&self) -> Result<impl Iterator<Item = u32> + '_, ()> {
        Ok(self.0.iter().copied())
    }
}

#[whichever::dispatch(Source)]
enum Kind {
    Few(Few),
    One(One),
}

fn main() {}
