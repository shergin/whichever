//! The carrier that the `#[dispatchable]` of 0.4.0 wrote, next to a trait
//! written by hand: `implement!` keeps reading the message of that release.

pub trait Size {
    fn size(&self) -> usize;
}

#[allow(unused_macros)]
macro_rules! __whichever_Size_of_0_4_0 {
    (@v1 $facade:ident $($rest:tt)*) => {
        ::$facade::__private::implement! {
            @v1 trait { pub trait Size { fn size(&self) -> usize; } } $($rest)*
        }
    };
}

#[allow(unused_imports)]
pub(crate) use __whichever_Size_of_0_4_0 as Size;

pub struct Fixed(pub usize);

impl Size for Fixed {
    fn size(&self) -> usize {
        self.0
    }
}

#[whichever::dispatch(Size)]
pub enum Any {
    Fixed(Fixed),
}

#[test]
fn it_is_still_read() {
    assert_eq!(Any::Fixed(Fixed(3)).size(), 3);
}

/// The carrier that the `#[dispatchable]` of 0.6.0 wrote: signatures only,
/// under `v1`, with the provided methods named beside them. The reader
/// still leaves a provided method without a receiver to its default.
pub trait Counted {
    fn count(&self) -> usize;
    fn unit() -> usize {
        1
    }
}

#[allow(unused_macros)]
macro_rules! __whichever_Counted_of_0_6_0 {
    (@v1 $facade:ident $($rest:tt)*) => {
        ::$facade::__private::implement! {
            @v1 trait { pub trait Counted { fn count(&self) -> usize; fn unit() -> usize; } } provided { unit } $($rest)*
        }
    };
}

#[allow(unused_imports)]
pub(crate) use __whichever_Counted_of_0_6_0 as Counted;

pub struct Three;

impl Counted for Three {
    fn count(&self) -> usize {
        3
    }
}

#[whichever::dispatch(Counted)]
pub enum AnyCounted {
    Three(Three),
}

#[test]
fn the_signatures_only_carrier_of_the_last_release_is_still_read() {
    assert_eq!(AnyCounted::Three(Three).count(), 3);
    assert_eq!(<AnyCounted as Counted>::unit(), 1);
}
