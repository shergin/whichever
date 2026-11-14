//! An arm reaches its target through a route: a field of the payload, what
//! a method returns, or the marked field of a variant with several; per
//! variant, per trait, or per inherent method.

use std::fmt::{self, Display, Formatter};
use std::pin::Pin;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Handle(pub u64);

pub struct Dictionary {
    pub handle: Handle,
}

// The marked field of a variant with several, beside a routed tuple
// variant: `value` reads the field through the binding where the return
// type asks for a value, and hands the binding over where it asks for a
// reference, so each arm has the type the routed arm has.
#[whichever::dispatch(from = false)]
pub enum Object {
    #[dispatch(via = .handle)]
    Dictionary(Dictionary),
    Tagged(#[dispatch] Handle, u32),
    Unknown {
        name: String,
        #[dispatch]
        handle: Handle,
    },
}

/// A lift other than `Some` or `Ok`, whose parameter is the reference the
/// pattern binds.
fn raw(handle: &Handle) -> u64 {
    handle.0
}

#[whichever::dispatch]
impl Object {
    /// The handle by value: `inner.handle` and `*inner`.
    #[dispatch(*, value)]
    pub fn handle(&self) -> Handle;

    /// The handle by reference: `&inner.handle` and `inner`.
    #[dispatch(*, value)]
    pub fn handle_ref(&self) -> &Handle;

    /// The exclusive form: `&mut inner.handle` and `inner`.
    #[dispatch(*, value)]
    pub fn handle_mut(&mut self) -> &mut Handle;

    /// By value under an exclusive receiver.
    #[dispatch(*, value)]
    pub fn handle_through_mut(&mut self) -> Handle;

    /// By value under `Some`, whose first argument is not a reference.
    #[dispatch(*, value, with = Some)]
    pub fn some_handle(&self) -> Option<Handle>;

    /// By reference under `Some`, whose first argument is one.
    #[dispatch(*, value, with = Some)]
    pub fn some_handle_ref(&self) -> Option<&Handle>;

    /// By value under `Ok`.
    #[dispatch(*, value, with = Ok)]
    pub fn ok_handle(&self) -> Result<Handle, ()>;

    /// A receiver by value, under which the binding is the field itself.
    #[dispatch(*, value)]
    pub fn into_handle(self) -> Handle;

    /// Any other lift is handed the binding as it stands.
    #[dispatch(Tagged, Unknown, value, with = raw)]
    pub fn raw_of_marked(&self) -> u64 {
        0
    }

    /// By value under a pinned receiver, which binds references too.
    #[dispatch(*, value)]
    pub fn pinned_handle(self: Pin<&mut Self>) -> Handle;
}

// An awaited marked field: the return type speaks of the future's output,
// so the binding is awaited as it stands.
#[whichever::dispatch(from = false)]
pub enum Lookup {
    Pending {
        id: u32,
        #[dispatch]
        ready: core::future::Ready<u64>,
    },
    Queued(u8, #[dispatch] core::future::Ready<u64>),
}

#[whichever::dispatch]
impl Lookup {
    #[dispatch(*, value)]
    pub async fn ready(&mut self) -> u64;
}

fn objects() -> [Object; 3] {
    [
        Object::Dictionary(Dictionary { handle: Handle(1) }),
        Object::Tagged(Handle(2), 20),
        Object::Unknown {
            name: "unknown".to_owned(),
            handle: Handle(3),
        },
    ]
}

#[test]
fn a_marked_field_is_read_by_value_and_borrowed_by_reference() {
    let mut objects = objects();
    let handles = objects.each_ref().map(Object::handle);
    assert_eq!(handles, [Handle(1), Handle(2), Handle(3)]);
    let handles = objects.each_ref().map(|object| *object.handle_ref());
    assert_eq!(handles, [Handle(1), Handle(2), Handle(3)]);
    let [dictionary, tagged, unknown] = &objects;
    let Object::Dictionary(inner) = dictionary else {
        unreachable!()
    };
    assert!(std::ptr::eq(dictionary.handle_ref(), &inner.handle));
    let Object::Tagged(field, _) = tagged else {
        unreachable!()
    };
    assert!(std::ptr::eq(tagged.handle_ref(), field));
    let Object::Unknown { handle: field, .. } = unknown else {
        unreachable!()
    };
    assert!(std::ptr::eq(unknown.handle_ref(), field));
    for object in &mut objects {
        object.handle_mut().0 += 10;
    }
    assert_eq!(
        objects.each_ref().map(Object::handle),
        [Handle(11), Handle(12), Handle(13)]
    );
    assert_eq!(
        objects.each_mut().map(Object::handle_through_mut),
        [Handle(11), Handle(12), Handle(13)]
    );
    assert_eq!(
        objects
            .each_mut()
            .map(|object| Pin::new(object).pinned_handle()),
        [Handle(11), Handle(12), Handle(13)]
    );
    assert!(matches!(objects[1], Object::Tagged(Handle(12), 20)));
    assert_eq!(
        objects.map(Object::into_handle),
        [Handle(11), Handle(12), Handle(13)]
    );
}

#[test]
fn a_marked_field_under_some_or_ok_is_read_as_the_first_argument_asks() {
    let objects = objects();
    assert_eq!(
        objects.each_ref().map(Object::some_handle),
        [Some(Handle(1)), Some(Handle(2)), Some(Handle(3))]
    );
    assert_eq!(
        objects.each_ref().map(Object::some_handle_ref),
        [Some(&Handle(1)), Some(&Handle(2)), Some(&Handle(3))]
    );
    let Object::Tagged(field, _) = &objects[1] else {
        unreachable!()
    };
    assert!(std::ptr::eq(objects[1].some_handle_ref().unwrap(), field));
    assert_eq!(
        objects.each_ref().map(Object::ok_handle),
        [Ok(Handle(1)), Ok(Handle(2)), Ok(Handle(3))]
    );
}

#[test]
fn any_other_lift_is_handed_the_marked_field_as_the_pattern_binds_it() {
    let objects = objects();
    assert_eq!(objects.each_ref().map(Object::raw_of_marked), [0, 2, 3]);
}

#[test]
fn an_awaited_marked_field_is_awaited_as_it_stands() {
    let mut pending = Lookup::Pending {
        id: 1,
        ready: core::future::ready(5),
    };
    assert_eq!(pollster::block_on(pending.ready()), 5);
    let mut queued = Lookup::Queued(2, core::future::ready(6));
    assert_eq!(pollster::block_on(queued.ready()), 6);
}
