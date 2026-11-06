//! An enum another crate defines is restated once, `remote` naming the real
//! enum, and a trait of this crate is forwarded over it; the real enum is
//! imported under the restatement's name for a partial block.

/// Stands for another crate.
pub mod other {
    pub enum Pair<L, R> {
        Left(L),
        Right(R),
    }
}

use other::Pair;

#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u32;
    fn scaled(&self, by: u32) -> Self;
}

#[whichever::dispatchable]
pub trait Named {
    fn name(&self) -> &'static str;
    fn kind() -> &'static str;
}

#[derive(Debug, PartialEq)]
pub struct Square(pub u32);
#[derive(Debug, PartialEq)]
pub struct Circle(pub u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn scaled(&self, by: u32) -> Self {
        Square(self.0 * by)
    }
}

impl Area for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
    fn scaled(&self, by: u32) -> Self {
        Circle(self.0 * by)
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

impl Named for Circle {
    fn name(&self) -> &'static str {
        "circle"
    }
    fn kind() -> &'static str {
        "shape"
    }
}

#[whichever::dispatch(remote = other::Pair)]
#[whichever::dispatch(Area)]
pub enum Pair<L, R> {
    Left(L),
    Right(R),
}

#[whichever::dispatch]
impl<L: Named, R: Named> Named for Pair<L, R> {
    fn kind() -> &'static str {
        "pair"
    }
}

#[test]
fn the_trait_is_forwarded_over_the_other_crates_enum() {
    let left: Pair<Square, Circle> = Pair::Left(Square(2));
    let right: Pair<Square, Circle> = Pair::Right(Circle(1));
    assert_eq!((left.area(), right.area()), (4, 3));
    assert!(matches!(left.scaled(3), Pair::Left(Square(6))));
    assert_eq!((left.name(), right.name()), ("square", "circle"));
    assert_eq!(<Pair<Square, Circle>>::kind(), "pair");
}
