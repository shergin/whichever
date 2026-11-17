use std::io;
use std::ops::Range;
use std::vec::IntoIter;
pub struct Tcp(pub Vec<u8>);
pub struct Memory(pub Vec<u8>);
impl io::Write for Tcp {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl io::Write for Memory {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
use __whichever_Write_6c7ff45f3e496a7c_28_1 as Write;
pub enum Stream {
    Tcp(Tcp),
    Memory(Memory),
}
#[automatically_derived]
impl ::core::convert::From<Tcp> for Stream {
    #[inline]
    fn from(__value: Tcp) -> Self {
        Self::Tcp(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Memory> for Stream {
    #[inline]
    fn from(__value: Memory) -> Self {
        Self::Memory(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Stream_2b72aa00a652485e_35_1 as Stream;
#[automatically_derived]
impl io::Write for Stream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Tcp(__inner) => <Tcp as io::Write>::write(__inner, buf),
            Stream::Memory(__inner) => <Memory as io::Write>::write(__inner, buf),
        }
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Tcp(__inner) => <Tcp as io::Write>::flush(__inner),
            Stream::Memory(__inner) => <Memory as io::Write>::flush(__inner),
        }
    }
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Stream::Tcp(__inner) => <Tcp as io::Write>::write_all(__inner, buf),
            Stream::Memory(__inner) => <Memory as io::Write>::write_all(__inner, buf),
        }
    }
}
pub enum ClientError {
    Io(io::Error),
    Message(String),
}
#[automatically_derived]
impl ::core::convert::From<io::Error> for ClientError {
    #[inline]
    fn from(__value: io::Error) -> Self {
        Self::Io(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_ClientError_97504760822228e9_43_1 as ClientError;
#[automatically_derived]
impl ::core::fmt::Display for ClientError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
            ClientError::Io(__inner) => {
                <io::Error as ::core::fmt::Display>::fmt(__inner, f)
            }
            ClientError::Message(__inner) => {
                <String as ::core::fmt::Display>::fmt(__inner, f)
            }
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for ClientError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Self::Io(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Io", &__self_0)
            }
            Self::Message(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "Message",
                    &__self_0,
                )
            }
        }
    }
}
pub enum Numbers {
    Range(Range<u32>),
    Owned(IntoIter<u32>),
}
#[automatically_derived]
impl ::core::convert::From<Range<u32>> for Numbers {
    #[inline]
    fn from(__value: Range<u32>) -> Self {
        Self::Range(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<IntoIter<u32>> for Numbers {
    #[inline]
    fn from(__value: IntoIter<u32>) -> Self {
        Self::Owned(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Numbers_767f150e5494ae65_50_1 as Numbers;
#[automatically_derived]
impl ::core::iter::Iterator for Numbers {
    type Item = <Range<u32> as ::core::iter::Iterator>::Item;
    #[inline]
    fn next(&mut self) -> ::core::option::Option<Self::Item> {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::next(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::next(__inner)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, ::core::option::Option<usize>) {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::size_hint(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::size_hint(__inner)
            }
        }
    }
    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::count(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::count(__inner)
            }
        }
    }
    #[inline]
    fn last(self) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::last(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::last(__inner)
            }
        }
    }
    #[inline]
    fn nth(&mut self, n: usize) -> ::core::option::Option<Self::Item> {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::nth(__inner, n)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::nth(__inner, n)
            }
        }
    }
    #[inline]
    fn for_each<F>(self, f: F)
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item),
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::for_each(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::for_each(__inner, f)
            }
        }
    }
    #[inline]
    fn collect<B: ::core::iter::FromIterator<Self::Item>>(self) -> B
    where
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::collect::<B>(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::collect::<B>(__inner)
            }
        }
    }
    #[inline]
    fn partition<B, F>(self, f: F) -> (B, B)
    where
        Self: Sized,
        B: ::core::default::Default + ::core::iter::Extend<Self::Item>,
        F: ::core::ops::FnMut(&Self::Item) -> bool,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::partition::<B, F>(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::partition::<B, F>(__inner, f)
            }
        }
    }
    #[inline]
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: ::core::ops::FnMut(B, Self::Item) -> B,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::fold(__inner, init, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::fold(__inner, init, f)
            }
        }
    }
    #[inline]
    fn reduce<F>(self, f: F) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item, Self::Item) -> Self::Item,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::reduce(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::reduce(__inner, f)
            }
        }
    }
    #[inline]
    fn all<F>(&mut self, f: F) -> bool
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item) -> bool,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::all(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::all(__inner, f)
            }
        }
    }
    #[inline]
    fn any<F>(&mut self, f: F) -> bool
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item) -> bool,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::any(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::any(__inner, f)
            }
        }
    }
    #[inline]
    fn find<P>(&mut self, predicate: P) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        P: ::core::ops::FnMut(&Self::Item) -> bool,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::find(__inner, predicate)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::find(__inner, predicate)
            }
        }
    }
    #[inline]
    fn find_map<B, F>(&mut self, f: F) -> ::core::option::Option<B>
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item) -> ::core::option::Option<B>,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::find_map::<B, F>(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::find_map::<B, F>(__inner, f)
            }
        }
    }
    #[inline]
    fn position<P>(&mut self, predicate: P) -> ::core::option::Option<usize>
    where
        Self: Sized,
        P: ::core::ops::FnMut(Self::Item) -> bool,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::position(__inner, predicate)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::position(__inner, predicate)
            }
        }
    }
    #[inline]
    fn max(self) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        Self::Item: ::core::cmp::Ord,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::max(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::max(__inner)
            }
        }
    }
    #[inline]
    fn min(self) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        Self::Item: ::core::cmp::Ord,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::min(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::min(__inner)
            }
        }
    }
    #[inline]
    fn max_by_key<B: ::core::cmp::Ord, F>(
        self,
        f: F,
    ) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item) -> B,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::max_by_key::<B, F>(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::max_by_key::<B, F>(__inner, f)
            }
        }
    }
    #[inline]
    fn max_by<F>(self, compare: F) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item, &Self::Item) -> ::core::cmp::Ordering,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::max_by(__inner, compare)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::max_by(__inner, compare)
            }
        }
    }
    #[inline]
    fn min_by_key<B: ::core::cmp::Ord, F>(
        self,
        f: F,
    ) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item) -> B,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::min_by_key::<B, F>(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::min_by_key::<B, F>(__inner, f)
            }
        }
    }
    #[inline]
    fn min_by<F>(self, compare: F) -> ::core::option::Option<Self::Item>
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item, &Self::Item) -> ::core::cmp::Ordering,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::min_by(__inner, compare)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::min_by(__inner, compare)
            }
        }
    }
    #[inline]
    fn sum<S>(self) -> S
    where
        Self: Sized,
        S: ::core::iter::Sum<Self::Item>,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::sum::<S>(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::sum::<S>(__inner)
            }
        }
    }
    #[inline]
    fn product<P>(self) -> P
    where
        Self: Sized,
        P: ::core::iter::Product<Self::Item>,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::product::<P>(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::product::<P>(__inner)
            }
        }
    }
    #[inline]
    fn cmp<I>(self, other: I) -> ::core::cmp::Ordering
    where
        I: ::core::iter::IntoIterator<Item = Self::Item>,
        Self::Item: ::core::cmp::Ord,
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::cmp(__inner, other)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::cmp(__inner, other)
            }
        }
    }
    #[inline]
    fn partial_cmp<I>(self, other: I) -> ::core::option::Option<::core::cmp::Ordering>
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::partial_cmp(__inner, other)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::partial_cmp(__inner, other)
            }
        }
    }
    #[inline]
    fn eq<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialEq<I::Item>,
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::eq(__inner, other)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::eq(__inner, other)
            }
        }
    }
    #[inline]
    fn ne<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialEq<I::Item>,
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::ne(__inner, other)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::ne(__inner, other)
            }
        }
    }
    #[inline]
    fn lt<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::lt(__inner, other)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::lt(__inner, other)
            }
        }
    }
    #[inline]
    fn le<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::le(__inner, other)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::le(__inner, other)
            }
        }
    }
    #[inline]
    fn gt<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::gt(__inner, other)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::gt(__inner, other)
            }
        }
    }
    #[inline]
    fn ge<I>(self, other: I) -> bool
    where
        I: ::core::iter::IntoIterator,
        Self::Item: ::core::cmp::PartialOrd<I::Item>,
        Self: Sized,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::ge(__inner, other)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::ge(__inner, other)
            }
        }
    }
    #[inline]
    fn is_sorted(self) -> bool
    where
        Self: Sized,
        Self::Item: ::core::cmp::PartialOrd,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::is_sorted(__inner)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::is_sorted(__inner)
            }
        }
    }
    #[inline]
    fn is_sorted_by<F>(self, compare: F) -> bool
    where
        Self: Sized,
        F: ::core::ops::FnMut(&Self::Item, &Self::Item) -> bool,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<u32> as ::core::iter::Iterator>::is_sorted_by(__inner, compare)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<u32> as ::core::iter::Iterator>::is_sorted_by(__inner, compare)
            }
        }
    }
    #[inline]
    fn is_sorted_by_key<F, K>(self, f: F) -> bool
    where
        Self: Sized,
        F: ::core::ops::FnMut(Self::Item) -> K,
        K: ::core::cmp::PartialOrd,
    {
        match self {
            Numbers::Range(__inner) => {
                <Range<
                    u32,
                > as ::core::iter::Iterator>::is_sorted_by_key::<F, K>(__inner, f)
            }
            Numbers::Owned(__inner) => {
                <IntoIter<
                    u32,
                > as ::core::iter::Iterator>::is_sorted_by_key::<F, K>(__inner, f)
            }
        }
    }
}
const _: () = {
    #[allow(dead_code)]
    struct __WhicheverAgreement(Numbers);
    #[allow(dead_code, non_snake_case)]
    impl __WhicheverAgreement {
        fn __first_implements<__First: ?Sized + ::core::iter::Iterator>() {}
        fn __same_as_first_variant<
            __Variant: ?Sized
                + ::core::iter::Iterator<
                    Item = <Range<u32> as ::core::iter::Iterator>::Item,
                >,
        >() {}
        fn __whichever_associated_types_agree() {
            Self::__first_implements::<Range<u32>>();
            Self::__same_as_first_variant::<IntoIter<u32>>();
        }
    }
};
fn main() {}
