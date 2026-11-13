//! A provided combinator that wraps `Self`, as rodio's `Source::buffered`
//! does, cannot be forwarded: the payload's `Buffered<Payload>` is not the
//! enum's. The way out is to leave it to the trait's default in a partial
//! block.
pub struct Buffered<S>(pub S);

#[whichever::dispatchable]
pub trait Source {
    fn channels(&self) -> u16;
    fn buffered(self) -> Buffered<Self>
    where
        Self: Sized,
    {
        Buffered(self)
    }
}

pub struct Mono;
pub struct Stereo;

impl Source for Mono {
    fn channels(&self) -> u16 {
        1
    }
}

impl Source for Stereo {
    fn channels(&self) -> u16 {
        2
    }
}

#[whichever::dispatch(Source)]
pub enum Any {
    Mono(Mono),
    Stereo(Stereo),
}

fn main() {}
