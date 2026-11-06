//! An arm reaches its target through a route: a field of the payload, what
//! a method returns, or the marked field of a variant with several; per
//! variant, per trait, or per inherent method.

use std::fmt::{self, Display, Formatter};

#[whichever::dispatchable]
pub trait Area {
    fn area(&self) -> u32;
    fn grow(&mut self, by: u32);
    fn into_area(self) -> u32;
}

#[whichever::dispatchable]
pub trait Named {
    fn name(&self) -> &'static str;
}

#[derive(Clone)]
pub struct Square(pub u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn grow(&mut self, by: u32) {
        self.0 += by;
    }
    fn into_area(self) -> u32 {
        self.0 * self.0
    }
}

impl Named for Square {
    fn name(&self) -> &'static str {
        "square"
    }
}

impl Square {
    /// An inherent method of the trait's name, which a routed arm must
    /// not pick: the call is qualified by the trait.
    pub fn area(&self) -> u32 {
        999
    }
}

pub struct Account {
    pub base: Square,
    pub id: u32,
}

impl Account {
    pub fn base(&self) -> &Square {
        &self.base
    }
}

pub struct Label(pub &'static str);

impl Named for Label {
    fn name(&self) -> &'static str {
        self.0
    }
}

// `Module` routes through a field for every trait; `Pair` through `.1`
// for `Area` and `.0` for `Named`; `Tagged` forwards to its marked field;
// `Row` to the field of its struct variant.
#[whichever::dispatch(Area, via(Pair = .1))]
#[whichever::dispatch(Named, via(Pair = .0))]
pub enum Any {
    #[dispatch(via = .base)]
    Module(Account),
    Pair((Label, Square)),
    Tagged(#[dispatch] Square, u32),
    Row {
        id: u32,
        #[dispatch]
        cell: Square,
    },
    Plain(Square),
}

// A route through a method, for `&self` methods.
#[whichever::dispatch(Named)]
pub enum Viewed {
    #[dispatch(via = .base())]
    Module(Account),
}

#[whichever::dispatch]
impl Any {
    /// A field of the target, after the variant's route, by value.
    #[dispatch(Module, Tagged, Row, Plain, field = 0)]
    pub fn side(&self) -> u32 {
        0
    }

    /// The same field by reference, as the return type asks.
    #[dispatch(Module, Tagged, Row, Plain, field = 0)]
    pub fn side_ref(&self) -> &u32 {
        &0
    }

    /// The square itself, widened to the trait object.
    #[dispatch(Plain, Tagged, value, with = Some)]
    pub fn as_named(&self) -> Option<&dyn Named> {
        None
    }

    /// The method's own route, over the variant's.
    #[dispatch(Module, via = .base(), call = name)]
    pub fn base_name(&self) -> &'static str {
        "none"
    }
}

impl Display for Any {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.area())
    }
}

#[test]
fn the_arms_follow_the_routes() {
    let mut shapes = vec![
        Any::Module(Account {
            base: Square(2),
            id: 7,
        }),
        Any::Pair((Label("pair"), Square(3))),
        Any::Tagged(Square(4), 1),
        Any::Row {
            id: 8,
            cell: Square(5),
        },
        Any::Plain(Square(6)),
    ];
    let areas: Vec<u32> = shapes.iter().map(Area::area).collect();
    assert_eq!(areas, [4, 9, 16, 25, 36]);
    let names: Vec<&str> = shapes.iter().map(Named::name).collect();
    assert_eq!(names, ["square", "pair", "square", "square", "square"]);
    for shape in &mut shapes {
        shape.grow(1);
    }
    assert_eq!(shapes[0].to_string(), "9");
    let sides: Vec<u32> = shapes.iter().map(Any::side).collect();
    assert_eq!(sides, [3, 0, 5, 6, 7]);
    let sides: Vec<u32> = shapes.iter().map(|shape| *shape.side_ref()).collect();
    assert_eq!(sides, [3, 0, 5, 6, 7]);
    assert_eq!(shapes[4].as_named().map(Named::name), Some("square"));
    assert_eq!(shapes[2].as_named().map(Named::name), Some("square"));
    assert!(shapes[1].as_named().is_none());
    assert_eq!(shapes[0].base_name(), "square");
    assert_eq!(shapes[1].base_name(), "none");
    let owned: Vec<u32> = shapes.into_iter().map(Area::into_area).collect();
    assert_eq!(owned, [9, 16, 25, 36, 49]);
    let viewed = Viewed::Module(Account {
        base: Square(1),
        id: 0,
    });
    assert_eq!(viewed.name(), "square");
}
