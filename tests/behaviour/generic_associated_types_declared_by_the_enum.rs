//! A generic associated type has no first variant to come from: the enum
//! declares it in a partial impl block, and every payload's converts into it.

use std::fmt::Debug;

#[whichever::dispatchable]
pub trait Sink<T> {
    fn put(&mut self, value: T);
    fn len(&self) -> usize;
}

#[whichever::dispatchable]
pub trait Make {
    type Out<T>: Sink<T>
    where
        T: Send;
    type View<'s>: Debug
    where
        Self: 's;
    fn make<T: Send>(&self) -> Self::Out<T>;
    fn view(&self) -> Self::View<'_>;
    fn name(&self) -> &'static str;
}

pub struct VecSink<T>(Vec<T>);

impl<T> Sink<T> for VecSink<T> {
    fn put(&mut self, value: T) {
        self.0.push(value);
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

pub struct CountSink(usize);

impl<T> Sink<T> for CountSink {
    fn put(&mut self, _: T) {
        self.0 += 1;
    }
    fn len(&self) -> usize {
        self.0
    }
}

pub struct A(u8);
pub struct B<'x>(&'x str);

impl Make for A {
    type Out<T>
        = VecSink<T>
    where
        T: Send;
    type View<'s>
        = u8
    where
        Self: 's;
    fn make<T: Send>(&self) -> VecSink<T> {
        VecSink(vec![])
    }
    fn view(&self) -> u8 {
        self.0
    }
    fn name(&self) -> &'static str {
        "a"
    }
}

impl<'x> Make for B<'x> {
    type Out<T>
        = CountSink
    where
        T: Send;
    type View<'s>
        = &'x str
    where
        Self: 's;
    fn make<T: Send>(&self) -> CountSink {
        CountSink(0)
    }
    fn view(&self) -> &'x str {
        self.0
    }
    fn name(&self) -> &'static str {
        "b"
    }
}

// The enum's own sink: a dispatch enum over `Sink<T>` at its own `T`.
#[whichever::dispatch(Sink<T>)]
pub enum AnySink<T> {
    Vec(VecSink<T>),
    Count(CountSink),
}

#[derive(Debug, PartialEq)]
pub enum AnyView<'s> {
    Num(u8),
    Str(&'s str),
}

impl From<u8> for AnyView<'_> {
    fn from(value: u8) -> Self {
        AnyView::Num(value)
    }
}

impl<'s> From<&'s str> for AnyView<'s> {
    fn from(value: &'s str) -> Self {
        AnyView::Str(value)
    }
}

#[whichever::dispatch(from = false)]
pub enum Kind<'x> {
    A(A),
    B(B<'x>),
}

#[whichever::dispatch]
impl<'x> Make for Kind<'x> {
    type Out<T>
        = AnySink<T>
    where
        T: Send;
    type View<'s>
        = AnyView<'s>
    where
        Self: 's;
}

#[test]
fn each_payloads_type_converts_into_the_declared_one() {
    let a = Kind::A(A(7));
    let text = String::from("text");
    let b = Kind::B(B(&text));
    let mut sink = a.make::<u32>();
    sink.put(1);
    sink.put(2);
    assert!(matches!(sink, AnySink::Vec(_)));
    assert_eq!(sink.len(), 2);
    let mut counted = b.make::<&str>();
    counted.put("x");
    assert!(matches!(counted, AnySink::Count(_)));
    assert_eq!(counted.len(), 1);
    assert_eq!(a.view(), AnyView::Num(7));
    assert_eq!(b.view(), AnyView::Str("text"));
    assert_eq!((a.name(), b.name()), ("a", "b"));
}
