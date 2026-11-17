//! A struct another crate defines is restated with `remote`, and the impls
//! are written for the real path. The restatement lists the fields it
//! forwards to, and the real struct may have more: its patterns end in
//! `..`. The real struct is imported under the restatement's name for a
//! partial block.

/// Stands for another crate.
pub mod other {
    #[derive(Debug, PartialEq)]
    pub struct Square(pub u32);

    #[derive(Debug, PartialEq)]
    pub struct Solo(pub Square);

    pub struct Pair {
        pub left: Square,
        pub right: u8,
        pub note: &'static str,
    }

    #[derive(Debug, PartialEq)]
    pub struct Single {
        pub shape: Square,
    }
}

use other::Square;

#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u32;
}

#[whichever::dispatchable]
pub trait Grown {
    fn grown(&self) -> Self;
}

#[whichever::dispatchable]
pub trait Tag {
    fn tag(&self) -> u8;
}

#[whichever::dispatchable]
pub trait Named {
    fn name(&self) -> &'static str;
    fn kind() -> &'static str;
}

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

impl Grown for Square {
    fn grown(&self) -> Self {
        Square(self.0 * 2)
    }
}

impl Tag for u8 {
    fn tag(&self) -> u8 {
        *self
    }
}

impl Named for Square {
    fn name(&self) -> &'static str {
        "square"
    }
    fn kind() -> &'static str {
        "shape"
    }
}

pub mod solo {
    use super::{Area, Grown, Square};

    #[whichever::dispatch(Area, remote = super::other::Solo)]
    #[whichever::dispatch(Grown)]
    pub struct Solo(Square);
}

pub mod pair {
    use super::{Area, Square, Tag};

    /// Lists the two fields its impls forward to, and leaves out `note`.
    #[whichever::dispatch(Area, remote = super::other::Pair)]
    #[whichever::dispatch(Tag)]
    pub struct Pair {
        #[dispatch]
        left: Square,
        #[dispatch(Tag)]
        right: u8,
    }
}

pub mod single {
    pub use super::other::Single;
    use super::{Grown, Named, Square};

    #[whichever::dispatch(Grown, remote = super::other::Single)]
    pub struct Single {
        shape: Square,
    }

    #[whichever::dispatch]
    impl Named for Single {
        fn kind() -> &'static str {
            "single"
        }
    }
}

#[test]
fn a_trait_is_forwarded_over_the_other_crates_tuple_struct() {
    let solo = other::Solo(Square(2));
    assert_eq!(solo.area(), 4);
    assert_eq!(solo.grown(), other::Solo(Square(4)));
}

#[test]
fn the_marked_field_and_the_field_naming_a_trait_are_reached_past_a_field_left_out() {
    let pair = other::Pair {
        left: Square(3),
        right: 7,
        note: "left out of the restatement",
    };
    assert_eq!((pair.area(), pair.tag()), (9, 7));
    assert_eq!(pair.note.len(), 27);
}

#[test]
fn a_partial_block_implements_a_trait_for_the_real_struct() {
    let single = single::Single { shape: Square(5) };
    assert_eq!(single.name(), "square");
    assert_eq!(<other::Single as Named>::kind(), "single");
    assert_eq!(single.grown(), other::Single { shape: Square(10) });
}
