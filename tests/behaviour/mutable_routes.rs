//! A method route lends as its method does, so a mutable borrow takes a
//! route of its own, `via_mut`: on a field or a variant, and beside a trait
//! keyed by variant. A method that borrows the target mutably follows it,
//! every other method follows `via`, and each `Self` argument takes the
//! route of its own borrow. A route on a method's own mark is followed
//! whatever the method borrows.

use std::cell::Cell;
use std::pin::Pin;

#[whichever::dispatchable]
pub trait AudioRead {
    fn position(&self) -> u64;
    fn read(&mut self, buffer: &mut [u8]) -> usize;
    fn absorb(&mut self, other: &Self);
    fn drain_into(&self, other: &mut Self);
    fn rewind(self: Pin<&mut Self>);
}

#[derive(Default)]
pub struct Source {
    pub position: u64,
}

impl AudioRead for Source {
    fn position(&self) -> u64 {
        self.position
    }
    fn read(&mut self, buffer: &mut [u8]) -> usize {
        self.position += buffer.len() as u64;
        buffer.len()
    }
    fn absorb(&mut self, other: &Self) {
        self.position += other.position;
    }
    fn drain_into(&self, other: &mut Self) {
        other.position += self.position;
    }
    fn rewind(mut self: Pin<&mut Self>) {
        self.position = 0;
    }
}

/// A getter pair, each of which counts the times it lends: the count is
/// what a route to the field would skip.
#[derive(Default)]
pub struct Warp {
    source: Source,
    pub lent_shared: Cell<u32>,
    pub lent_mutably: Cell<u32>,
}

impl Warp {
    pub fn source(&self) -> &Source {
        self.lent_shared.set(self.lent_shared.get() + 1);
        &self.source
    }
    pub fn source_mut(&mut self) -> &mut Source {
        self.lent_mutably.set(self.lent_mutably.get() + 1);
        &mut self.source
    }
}

#[whichever::dispatch(AudioRead)]
#[derive(Default)]
pub struct RegisteredAudio {
    #[dispatch(via = .source(), via_mut = .source_mut())]
    pub warp: Warp,
}

#[test]
fn a_struct_takes_the_mutable_getter_for_a_mutable_borrow_alone() {
    let mut audio = RegisteredAudio::default();
    assert_eq!(audio.read(&mut [0; 3]), 3);
    assert_eq!(audio.position(), 3);
    assert_eq!(audio.warp.lent_mutably.get(), 1);
    let mut other = RegisteredAudio::default();
    other.read(&mut [0; 4]);
    audio.absorb(&other);
    assert_eq!(audio.position(), 7);
    assert_eq!(audio.warp.lent_mutably.get(), 2);
    assert_eq!(other.warp.lent_mutably.get(), 1);
    audio.drain_into(&mut other);
    assert_eq!(other.position(), 11);
    assert_eq!(audio.warp.lent_mutably.get(), 2);
    assert_eq!(other.warp.lent_mutably.get(), 2);
    Pin::new(&mut audio).rewind();
    assert_eq!(audio.position(), 0);
    assert_eq!(audio.warp.lent_mutably.get(), 3);
}

/// The field's route serves the shared borrows, and a partial block gives
/// the one method that borrows mutably a route of its own on its mark.
#[whichever::dispatch]
#[derive(Default)]
pub struct Registered {
    #[dispatch(via = .source())]
    pub warp: Warp,
}

#[whichever::dispatchable]
pub trait Seek {
    fn at(&self) -> u64;
    fn skip(&mut self, by: u64);
}

impl Seek for Source {
    fn at(&self) -> u64 {
        self.position
    }
    fn skip(&mut self, by: u64) {
        self.position += by;
    }
}

#[whichever::dispatch]
impl Seek for Registered {
    #[dispatch(*, via = .source_mut())]
    fn skip(&mut self, by: u64);
}

/// The block's attribute gives the trait its route for a mutable borrow,
/// keyed by the field, beside the field's own route.
#[whichever::dispatch]
#[derive(Default)]
pub struct RegisteredByBlock {
    #[dispatch(via = .source())]
    pub warp: Warp,
}

#[whichever::dispatch(via_mut(warp = .source_mut()))]
impl Seek for RegisteredByBlock {}

#[test]
fn a_method_of_a_partial_block_follows_its_own_route() {
    let mut registered = Registered::default();
    registered.skip(5);
    assert_eq!(registered.at(), 5);
    assert_eq!(registered.warp.lent_mutably.get(), 1);
    let mut by_block = RegisteredByBlock::default();
    by_block.skip(2);
    assert_eq!(by_block.at(), 2);
    assert_eq!(by_block.warp.lent_mutably.get(), 1);
}

/// `Own` gives both routes itself; `Keyed` takes the trait's, whose `via`
/// replaces both of its own; so does `Replaced`, whose mutable getter the
/// trait's one route to the field leaves unused; `Plain` needs none.
#[whichever::dispatch(
    AudioRead,
    via(Keyed = .source(), Replaced = .source),
    via_mut(Keyed = .source_mut()),
    mismatch(absorb = (), drain_into = ())
)]
pub enum Track {
    #[dispatch(via = .source(), via_mut = .source_mut())]
    Own(Warp),
    #[dispatch(via = .source)]
    Keyed(Warp),
    #[dispatch(via = .source(), via_mut = .source_mut())]
    Replaced(Warp),
    Plain(Source),
}

fn lent(track: &Track) -> u32 {
    match track {
        Track::Own(warp) | Track::Keyed(warp) | Track::Replaced(warp) => warp.lent_mutably.get(),
        Track::Plain(_) => 0,
    }
}

#[test]
fn an_enum_follows_its_variants_routes_and_the_traits() {
    let mut tracks = [
        Track::Own(Warp::default()),
        Track::Keyed(Warp::default()),
        Track::Replaced(Warp::default()),
        Track::Plain(Source::default()),
    ];
    for track in &mut tracks {
        track.read(&mut [0; 2]);
    }
    assert_eq!(tracks.each_ref().map(AudioRead::position), [2, 2, 2, 2]);
    assert_eq!(tracks.each_ref().map(lent), [1, 1, 0, 0]);
    let [own, keyed, ..] = &mut tracks;
    let other = Track::Own(Warp::default());
    own.absorb(&other);
    keyed.drain_into(&mut Track::Keyed(Warp::default()));
    assert_eq!(lent(&other), 0);
    assert_eq!(tracks.each_ref().map(lent), [2, 1, 0, 0]);
}

pub struct Data(pub u8);

/// A getter pair over a message, whose mutable getter drops a cache before
/// it lends, as confluent-kafka's `MetadataResponse::data_mut` does.
pub struct Produce {
    data: Data,
}

impl Produce {
    pub fn data(&self) -> &Data {
        &self.data
    }
    pub fn data_mut(&mut self) -> &mut Data {
        &mut self.data
    }
}

pub struct Metadata {
    data: Data,
    pub cache: Option<u32>,
}

impl Metadata {
    pub fn data(&self) -> &Data {
        &self.data
    }
    pub fn data_mut(&mut self) -> &mut Data {
        self.cache = None;
        &mut self.data
    }
}

pub trait Message {
    fn encode(&mut self, version: u16) -> Vec<u8>;
}

impl Message for Data {
    fn encode(&mut self, version: u16) -> Vec<u8> {
        vec![self.0, version as u8]
    }
}

pub trait ToSend: Message {
    fn to_send(&mut self, version: u16) -> Vec<u8> {
        self.encode(version)
    }
}

impl<T: Message> ToSend for T {}

#[whichever::dispatch(from = false)]
pub enum Response {
    #[dispatch(via = .data(), via_mut = .data_mut())]
    Produce(Produce),
    #[dispatch(via = .data(), via_mut = .data_mut())]
    Metadata(Metadata),
}

#[whichever::dispatch]
impl Response {
    /// The place a route ending in a method reaches is that method's
    /// result, handed over as it is: `inner.data()`.
    #[dispatch(*, value)]
    pub fn data(&self) -> &Data;

    /// The exclusive form, `inner.data_mut()`.
    #[dispatch(*, value)]
    pub fn data_mut(&mut self) -> &mut Data;

    /// A field after the mutable route, `&mut inner.data_mut().0`.
    #[dispatch(*, field = 0)]
    pub fn tag_mut(&mut self) -> &mut u8;
}

/// The variants give no route; the method's own is followed though it
/// borrows mutably, the hand's arm `inner.data_mut().to_send(version)`.
#[whichever::dispatch(from = false)]
pub enum Request {
    Produce(Produce),
    Metadata(Metadata),
}

#[whichever::dispatch]
impl Request {
    #[dispatch(*, via = .data_mut(), call = to_send)]
    pub fn to_send(&mut self, version: u16) -> Vec<u8>;
}

#[test]
fn an_inherent_method_follows_its_own_route_through_the_mutable_getter() {
    let mut request = Request::Metadata(Metadata {
        data: Data(4),
        cache: Some(9),
    });
    assert_eq!(request.to_send(2), [4, 2]);
    assert!(matches!(
        request,
        Request::Metadata(Metadata { cache: None, .. })
    ));
    let mut produce = Request::Produce(Produce { data: Data(1) });
    assert_eq!(produce.to_send(3), [1, 3]);
}

#[test]
fn a_place_through_a_method_route_is_the_methods_result() {
    let mut response = Response::Metadata(Metadata {
        data: Data(5),
        cache: Some(1),
    });
    assert_eq!(response.data().0, 5);
    assert!(matches!(
        response,
        Response::Metadata(Metadata { cache: Some(1), .. })
    ));
    response.data_mut().0 = 6;
    assert!(matches!(
        response,
        Response::Metadata(Metadata { cache: None, .. })
    ));
    *response.tag_mut() += 1;
    assert_eq!(response.data().0, 7);
    let mut produce = Response::Produce(Produce { data: Data(0) });
    *produce.tag_mut() = 8;
    assert_eq!(produce.data().0, 8);
}

/// A trait's `via_mut` given alone replaces the variant's `via_mut` only:
/// `Kept`'s shared methods still follow its own `via`, its mutable ones the
/// trait's route to the field. A trait's `via` given alone replaces both:
/// `Replaced`'s mutable methods follow the trait's route to the field, and
/// neither of its getters is called.
#[whichever::dispatch(Seek, via_mut(Kept = .source), via(Replaced = .source))]
pub enum Split {
    #[dispatch(via = .source(), via_mut = .source_mut())]
    Kept(Warp),
    #[dispatch(via = .source(), via_mut = .source_mut())]
    Replaced(Warp),
}

fn lent_both(split: &Split) -> (u32, u32) {
    let (Split::Kept(warp) | Split::Replaced(warp)) = split;
    (warp.lent_shared.get(), warp.lent_mutably.get())
}

#[test]
fn a_traits_via_mut_alone_keeps_the_variants_via_and_its_via_replaces_both() {
    let mut kept = Split::Kept(Warp::default());
    let mut replaced = Split::Replaced(Warp::default());
    kept.skip(3);
    replaced.skip(4);
    assert_eq!((kept.at(), replaced.at()), (3, 4));
    assert_eq!(lent_both(&kept), (1, 0));
    assert_eq!(lent_both(&replaced), (0, 0));
}
