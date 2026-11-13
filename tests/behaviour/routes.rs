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

    /// A place reached through a route, borrowed as the return type asks:
    /// the arm `side_ref` writes for `Module` with `field = 0`.
    #[dispatch(Module, via = .base.0, value)]
    pub fn side_by_route(&self) -> &u32 {
        &0
    }
}

pub struct Card {
    pub id: String,
    pub n: u32,
    pub r: &'static u32,
}

// A place under a lift: `with = Some` hands the lift what the arm reaches,
// borrowed as the return type's first argument asks.
#[whichever::dispatch(from = false)]
pub enum Held {
    Boxed(Box<dyn Named>),
    Plain(Card),
    Other(Square),
}

/// What a bare `-> &String` returns for the variants it does not select.
static NO_ID: String = String::new();

#[whichever::dispatch]
impl Held {
    /// A field borrowed for `Some`, since the lift is handed a `&String`.
    #[dispatch(Plain, field = id, with = Some)]
    pub fn id_if_plain(&self) -> Option<&String> {
        None
    }

    /// The target a route reaches, borrowed for `Some` and widened to the
    /// trait object.
    #[dispatch(Boxed, via = *, value, with = Some)]
    pub fn as_named(&self) -> Option<&dyn Named> {
        None
    }

    /// The exclusive form: `Some(&mut inner.id)`.
    #[dispatch(Plain, field = id, with = Some)]
    pub fn id_if_plain_mut(&mut self) -> Option<&mut String> {
        None
    }

    /// A lift other than `Some`, borrowing for the `Ok` side.
    #[dispatch(Plain, field = id, with = Ok)]
    pub fn id_or_reason(&self) -> Result<&String, &'static str> {
        Err("not plain")
    }

    /// The field by reference without a lift.
    #[dispatch(Plain, field = id)]
    pub fn id(&self) -> &String {
        &NO_ID
    }

    /// A `Copy` field by value under a lift: the first argument is not a
    /// reference, so nothing is borrowed.
    #[dispatch(Plain, field = n, with = Some)]
    pub fn n_if_plain(&self) -> Option<u32> {
        None
    }

    /// The payload itself under a lift: the binding is already a
    /// reference and is not borrowed twice.
    #[dispatch(Plain, value, with = Some)]
    pub fn card(&self) -> Option<&Card> {
        None
    }

    /// A field that is itself a reference, under a lift through a trait:
    /// the place is handed over as it is, `Option::from(inner.r)`, since
    /// the trait would refuse a `&&u32`.
    #[dispatch(Plain, field = r, with = Option::from)]
    pub fn r_from(&self) -> Option<&u32> {
        None
    }

    /// The same field under `Some`, which borrows it: rustc takes the
    /// `&&u32` the arm hands `Some` as the `&u32` the return type asks for.
    #[dispatch(Plain, field = r, with = Some)]
    pub fn r_if_plain(&self) -> Option<&u32> {
        None
    }

    /// A call under a lift, which is never borrowed.
    #[dispatch(Boxed, call = as_ref, with = Some)]
    pub fn as_named_by_call(&self) -> Option<&dyn Named> {
        None
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
    let sides: Vec<u32> = shapes.iter().map(|shape| *shape.side_by_route()).collect();
    assert_eq!(sides, [3, 0, 0, 0, 0]);
    let owned: Vec<u32> = shapes.into_iter().map(Area::into_area).collect();
    assert_eq!(owned, [9, 16, 25, 36, 49]);
    let viewed = Viewed::Module(Account {
        base: Square(1),
        id: 0,
    });
    assert_eq!(viewed.name(), "square");
}

fn held() -> [Held; 3] {
    [
        Held::Boxed(Box::new(Label("boxed"))),
        Held::Plain(Card {
            id: "card".to_owned(),
            n: 3,
            r: &5,
        }),
        Held::Other(Square(2)),
    ]
}

#[test]
fn a_place_under_a_lift_is_borrowed_as_the_lift_is_handed() {
    let [boxed, plain, other] = held();
    assert_eq!(plain.id_if_plain().map(String::as_str), Some("card"));
    assert!(core::ptr::eq(
        plain.id_if_plain().unwrap(),
        match &plain {
            Held::Plain(card) => &card.id,
            _ => unreachable!(),
        }
    ));
    assert!(boxed.id_if_plain().is_none());
    assert!(other.id_if_plain().is_none());
    assert_eq!(boxed.as_named().map(Named::name), Some("boxed"));
    assert!(plain.as_named().is_none());
    assert!(other.as_named().is_none());
    assert_eq!(plain.id_or_reason().map(String::as_str), Ok("card"));
    assert_eq!(boxed.id_or_reason(), Err("not plain"));
    assert_eq!(other.id_or_reason(), Err("not plain"));
}

#[test]
fn an_exclusive_place_under_a_lift_is_mutated_through_the_result() {
    let [mut boxed, mut plain, mut other] = held();
    plain
        .id_if_plain_mut()
        .expect("the selected variant")
        .push_str("ed");
    assert_eq!(plain.id(), "carded");
    assert!(boxed.id_if_plain_mut().is_none());
    assert!(other.id_if_plain_mut().is_none());
}

#[test]
fn the_places_that_need_no_borrow_through_a_lift_are_unchanged() {
    let [boxed, plain, other] = held();
    assert_eq!(plain.id(), "card");
    assert_eq!(boxed.id(), "");
    assert_eq!(other.id(), "");
    assert_eq!(plain.n_if_plain(), Some(3));
    assert_eq!(other.n_if_plain(), None);
    assert_eq!(plain.card().map(|card| card.n), Some(3));
    assert!(boxed.card().is_none());
    assert_eq!(boxed.as_named_by_call().map(Named::name), Some("boxed"));
    assert!(plain.as_named_by_call().is_none());
}

#[test]
fn a_field_that_is_a_reference_is_handed_to_any_lift() {
    let [boxed, plain, other] = held();
    assert_eq!(plain.r_from(), Some(&5));
    assert_eq!(boxed.r_from(), None);
    assert_eq!(other.r_from(), None);
    assert_eq!(plain.r_if_plain(), Some(&5));
    assert_eq!(boxed.r_if_plain(), None);
    assert_eq!(other.r_if_plain(), None);
}
