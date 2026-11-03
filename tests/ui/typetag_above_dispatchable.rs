//! A trait reaching `#[dispatchable]` with typetag's hidden methods already
//! added has `#[typetag::serde]` above it; a forwarder of those methods
//! would serialize a wrapper under its payload's tag. The methods are
//! written by hand here, as typetag writes them.
#[whichever::dispatchable]
trait Piece {
    fn label(&self) -> String;
    #[doc(hidden)]
    fn typetag_name(&self) -> &'static str;
    #[doc(hidden)]
    fn typetag_deserialize(&self);
}

fn main() {}
