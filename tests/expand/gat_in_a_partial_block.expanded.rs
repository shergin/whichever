//! A generic associated type declared in the enum's partial impl block:
//! the arms convert each payload's own type into the declared one, as for
//! a plain override.
pub trait Lend {
    type Item<'a> where Self: 'a;
    fn lend(&self) -> Self::Item<'_>;
    fn len(&self) -> usize;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Lend_83de6e09edf2f454_4_1 as Lend;
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
pub enum Content {
    Text(Text),
    Bytes(Bytes),
}
#[automatically_derived]
impl ::core::convert::From<Text> for Content {
    #[inline]
    fn from(__value: Text) -> Self {
        Self::Text(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Bytes> for Content {
    #[inline]
    fn from(__value: Bytes) -> Self {
        Self::Bytes(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Content_add3accec8d6e405_53_1 as Content;
impl Lend for Content {
    #[inline]
    fn lend(&self) -> Self::Item<'_> {
        match self {
            Content::Text(__inner) => {
                ::core::convert::Into::into(<Text as Lend>::lend(__inner))
            }
            Content::Bytes(__inner) => {
                ::core::convert::Into::into(<Bytes as Lend>::lend(__inner))
            }
        }
    }
    #[inline]
    fn len(&self) -> usize {
        match self {
            Content::Text(__inner) => <Text as Lend>::len(__inner),
            Content::Bytes(__inner) => <Bytes as Lend>::len(__inner),
        }
    }
    type Item<'a> = AnyItem<'a> where Self: 'a;
}
fn main() {}
