//! A generic associated type declared in the enum's partial impl block:
//! the arms convert each payload's own type into the declared one, as for
//! a plain override.
#[whichever::dispatchable]
pub trait Lend {
    type Item<'a>
    where
        Self: 'a;
    fn lend(&self) -> Self::Item<'_>;
    fn len(&self) -> usize;
}

pub struct Text(pub String);
pub struct Bytes(pub Vec<u8>);

impl Lend for Text {
    type Item<'a> = &'a str;
    fn lend(&self) -> &str {
        &self.0
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl Lend for Bytes {
    type Item<'a> = &'a [u8];
    fn lend(&self) -> &[u8] {
        &self.0
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

pub enum AnyItem<'a> {
    Str(&'a str),
    Bytes(&'a [u8]),
}

impl<'a> From<&'a str> for AnyItem<'a> {
    fn from(value: &'a str) -> Self {
        AnyItem::Str(value)
    }
}

impl<'a> From<&'a [u8]> for AnyItem<'a> {
    fn from(value: &'a [u8]) -> Self {
        AnyItem::Bytes(value)
    }
}

#[whichever::dispatch]
pub enum Content {
    Text(Text),
    Bytes(Bytes),
}

#[whichever::dispatch]
impl Lend for Content {
    type Item<'a> = AnyItem<'a> where Self: 'a;
}

fn main() {}
