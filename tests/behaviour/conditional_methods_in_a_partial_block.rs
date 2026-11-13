//! A method a partial block writes under `#[cfg]` is the block's where the
//! `cfg` holds and forwarded where it does not, so the payload's override is
//! never hidden by the trait's default. A method the block marks with
//! `#[dispatch(..)]` under `#[cfg]` follows its `cfg` the same way, and the
//! builds it leaves go to a mark without a `cfg` where there is one.

#[whichever::dispatchable]
pub trait Tree {
    fn get(&self) -> u32;
    fn name(&self) -> &'static str {
        "default"
    }
    fn kind(&self) -> &'static str {
        "default"
    }
}

pub struct Inner;

impl Tree for Inner {
    fn get(&self) -> u32 {
        1
    }
    fn name(&self) -> &'static str {
        "inner"
    }
    fn kind(&self) -> &'static str {
        "inner"
    }
}

pub struct Other;

impl Tree for Other {
    fn get(&self) -> u32 {
        2
    }
}

#[whichever::dispatch]
pub enum AnyTree {
    Inner(Inner),
    Other(Other),
}

// An integration test is compiled with `cfg(test)` set, so `not(test)`
// is the arm that is off and `test` the one that is on.
#[whichever::dispatch]
impl Tree for AnyTree {
    #[cfg(not(test))]
    fn name(&self) -> &'static str {
        "enum"
    }

    #[cfg(test)]
    fn kind(&self) -> &'static str {
        "enum"
    }
}

#[whichever::dispatch]
pub struct Wrapped {
    #[dispatch]
    inner: Inner,
    pub label: u8,
}

#[whichever::dispatch]
impl Tree for Wrapped {
    #[cfg(not(test))]
    fn name(&self) -> &'static str {
        "wrapped"
    }
}

#[test]
fn the_payload_answers_where_the_cfg_is_off() {
    assert_eq!(AnyTree::Inner(Inner).name(), "inner");
    assert_eq!(AnyTree::Other(Other).name(), "default");
    assert_eq!(AnyTree::Inner(Inner).kind(), "enum");
    assert_eq!(AnyTree::Inner(Inner).get(), 1);
    let wrapped = Wrapped {
        inner: Inner,
        label: 1,
    };
    assert_eq!(
        (wrapped.name(), wrapped.get(), wrapped.label),
        ("inner", 1, 1)
    );
}

#[whichever::dispatch]
pub struct Left {
    #[dispatch]
    inner: Inner,
}

// A struct's `#[dispatch(default)]` under `#[cfg]` follows its `cfg` as an
// enum's does.
#[whichever::dispatch]
impl Tree for Left {
    #[cfg(not(test))]
    #[dispatch(default)]
    fn name(&self) -> &'static str;

    #[cfg(test)]
    #[dispatch(default)]
    fn kind(&self) -> &'static str;
}

#[test]
fn a_struct_leaves_a_method_to_the_default_only_where_the_cfg_holds() {
    let left = Left { inner: Inner };
    assert_eq!(
        (left.name(), left.kind(), left.get()),
        ("inner", "default", 1)
    );
}

/// A trait whose every provided method both payloads override, so that a
/// method wrongly left to the default answers `"shape"` where a payload's
/// answer is due.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
    fn label(&self) -> &'static str {
        "shape"
    }
    fn kind(&self) -> &'static str {
        "shape"
    }
    fn size(&self) -> &'static str {
        "shape"
    }
    fn tag(&self) -> &'static str {
        "shape"
    }
    fn flag(&self) -> &'static str {
        "shape"
    }
    fn note(&self) -> &'static str {
        "shape"
    }
    fn hint(&self) -> &'static str {
        "shape"
    }
}

pub struct Circle;

impl Shape for Circle {
    fn area(&self) -> u32 {
        3
    }
    fn name(&self) -> &'static str {
        "circle"
    }
    fn label(&self) -> &'static str {
        "circle"
    }
    fn kind(&self) -> &'static str {
        "circle"
    }
    fn size(&self) -> &'static str {
        "circle"
    }
    fn tag(&self) -> &'static str {
        "circle"
    }
    fn flag(&self) -> &'static str {
        "circle"
    }
    fn note(&self) -> &'static str {
        "circle"
    }
    fn hint(&self) -> &'static str {
        "circle"
    }
}

pub struct Square;

impl Shape for Square {
    fn area(&self) -> u32 {
        4
    }
    fn name(&self) -> &'static str {
        "square"
    }
    fn label(&self) -> &'static str {
        "square"
    }
    fn kind(&self) -> &'static str {
        "square"
    }
    fn size(&self) -> &'static str {
        "square"
    }
    fn tag(&self) -> &'static str {
        "square"
    }
    fn flag(&self) -> &'static str {
        "square"
    }
    fn note(&self) -> &'static str {
        "square"
    }
    fn hint(&self) -> &'static str {
        "square"
    }
}

#[whichever::dispatch]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}

// Each kind of mark is written once under `cfg(test)`, which holds, and
// once under `cfg(not(test))`, which does not, so both builds are seen in
// one.
#[whichever::dispatch]
impl Shape for AnyShape {
    // A required method left to a default it does not have is forwarded in
    // the builds its `cfg` leaves.
    #[cfg(not(test))]
    #[dispatch(default)]
    fn area(&self) -> u32;

    #[cfg(test)]
    #[dispatch(default)]
    fn name(&self) -> &'static str;

    #[cfg(not(test))]
    #[dispatch(default)]
    fn label(&self) -> &'static str;

    #[cfg(test)]
    #[dispatch(Circle)]
    fn kind(&self) -> &'static str {
        "body"
    }

    #[cfg(not(test))]
    #[dispatch(Circle)]
    fn size(&self) -> &'static str {
        "body"
    }

    // Two marks of one name under opposite `cfg`s, in both orders.
    #[cfg(test)]
    #[dispatch(Circle)]
    fn tag(&self) -> &'static str {
        "on"
    }

    #[cfg(not(test))]
    #[dispatch(Square)]
    fn tag(&self) -> &'static str {
        "off"
    }

    #[cfg(not(test))]
    #[dispatch(Circle)]
    fn flag(&self) -> &'static str {
        "off"
    }

    #[cfg(test)]
    #[dispatch(Square)]
    fn flag(&self) -> &'static str {
        "on"
    }

    // A mark without a `cfg` takes the builds a written method's `cfg`
    // leaves.
    #[cfg(test)]
    fn note(&self) -> &'static str {
        "written"
    }

    #[dispatch(Circle)]
    fn note(&self) -> &'static str {
        "body"
    }

    #[cfg(not(test))]
    fn hint(&self) -> &'static str {
        "written"
    }

    #[dispatch(Circle)]
    fn hint(&self) -> &'static str {
        "body"
    }
}

fn both() -> (AnyShape, AnyShape) {
    (AnyShape::Circle(Circle), AnyShape::Square(Square))
}

#[test]
fn a_required_method_left_to_the_default_where_the_cfg_is_off_is_forwarded() {
    let (circle, square) = both();
    assert_eq!((circle.area(), square.area()), (3, 4));
}

#[test]
fn a_default_mark_leaves_the_method_to_the_default_where_the_cfg_holds() {
    let (circle, square) = both();
    assert_eq!((circle.name(), square.name()), ("shape", "shape"));
}

#[test]
fn a_default_mark_is_forwarded_where_the_cfg_is_off() {
    let (circle, square) = both();
    assert_eq!((circle.label(), square.label()), ("circle", "square"));
}

#[test]
fn a_forwarded_mark_uses_its_body_where_the_cfg_holds() {
    let (circle, square) = both();
    assert_eq!((circle.kind(), square.kind()), ("circle", "body"));
}

#[test]
fn a_forwarded_mark_is_forwarded_to_every_variant_where_the_cfg_is_off() {
    let (circle, square) = both();
    assert_eq!((circle.size(), square.size()), ("circle", "square"));
}

#[test]
fn of_two_marks_under_opposite_cfgs_the_one_that_holds_is_written() {
    let (circle, square) = both();
    assert_eq!((circle.tag(), square.tag()), ("circle", "on"));
    assert_eq!((circle.flag(), square.flag()), ("on", "square"));
}

#[test]
fn a_mark_without_a_cfg_takes_the_builds_a_written_method_leaves() {
    let (circle, square) = both();
    assert_eq!((circle.note(), square.note()), ("written", "written"));
    assert_eq!((circle.hint(), square.hint()), ("circle", "body"));
}
