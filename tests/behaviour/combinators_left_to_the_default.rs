//! A provided combinator that wraps `Self` is left to the trait's default.
//! The shape is rodio's `Source`: a required method beside provided
//! combinators such as `fn buffered(self) -> Buffered<Self>`, whose payload
//! result `Buffered<Payload>` cannot be wrapped back into the enum. A partial
//! block leaves the combinator to the default, which wraps the enum itself.

/// A source wrapped by a combinator, with a mark of who wrapped it.
pub struct Buffered<S> {
    pub inner: S,
    pub by_payload: bool,
}

#[whichever::dispatchable]
pub trait Source {
    fn channels(&self) -> u16;
    fn buffered(self) -> Buffered<Self>
    where
        Self: Sized,
    {
        Buffered {
            inner: self,
            by_payload: false,
        }
    }
}

pub struct Mono;
pub struct Stereo;

impl Source for Mono {
    fn channels(&self) -> u16 {
        1
    }
}

/// A payload that buffers in its own way, which the enum's default skips.
impl Source for Stereo {
    fn channels(&self) -> u16 {
        2
    }
    fn buffered(self) -> Buffered<Self> {
        Buffered {
            inner: self,
            by_payload: true,
        }
    }
}

#[whichever::dispatch]
pub enum Any {
    Mono(Mono),
    Stereo(Stereo),
}

#[whichever::dispatch]
impl Source for Any {
    #[dispatch(default)]
    fn buffered(self) -> Buffered<Self>;
}

#[test]
fn the_required_method_is_forwarded_to_each_variant() {
    assert_eq!(Any::Mono(Mono).channels(), 1);
    assert_eq!(Any::Stereo(Stereo).channels(), 2);
}

#[test]
fn the_combinator_wraps_the_enum_and_still_forwards() {
    let buffered: Buffered<Any> = Any::Stereo(Stereo).buffered();
    assert_eq!(buffered.inner.channels(), 2);
    assert!(matches!(buffered.inner, Any::Stereo(Stereo)));
    let buffered: Buffered<Any> = Any::Mono(Mono).buffered();
    assert_eq!(buffered.inner.channels(), 1);
}

#[test]
fn the_default_runs_rather_than_the_payloads_override() {
    assert!(Stereo.buffered().by_payload);
    assert!(!Any::Stereo(Stereo).buffered().by_payload);
    assert!(!Any::Mono(Mono).buffered().by_payload);
}
