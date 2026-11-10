//! An inherent method forwarded to a `deref` variant, or through a route
//! that ends in a deref, calls the method of what the payload points at, and
//! a `field` or `value` arm reads that target; a method the pointer has of
//! the same name never takes over. Every case sits beside a plain variant.

use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct Circle {
    pub radius: u32,
}

impl Circle {
    pub fn describe(&self) -> String {
        format!("circle {}", self.radius)
    }
    pub fn grow(&mut self, by: u32) {
        self.radius += by;
    }
    pub fn into_radius(self) -> u32 {
        self.radius
    }
}

/// A circle behind a field, which a route reaches with `.boxed*`.
#[derive(Debug, PartialEq)]
pub struct Holder {
    pub boxed: Box<Circle>,
}

// Every pointer `deref` reaches through. Only shared access goes through an
// `Rc` or a shared reference, so the methods here take `&self`.
#[whichever::dispatch]
pub enum Shape<'a> {
    #[dispatch(deref)]
    Boxed(Box<Circle>),
    #[dispatch(deref)]
    Shared(Rc<Circle>),
    #[dispatch(deref)]
    Borrowed(&'a Circle),
    Plain(Circle),
}

#[whichever::dispatch]
impl<'a> Shape<'a> {
    #[dispatch(*)]
    pub fn describe(&self) -> String;

    #[dispatch(*, field = radius)]
    pub fn radius(&self) -> &u32;

    #[dispatch(*, field = radius)]
    pub fn radius_by_value(&self) -> u32;

    #[dispatch(*, value)]
    pub fn circle(&self) -> &Circle;
}

#[test]
fn a_shared_method_reaches_what_every_pointer_points_at() {
    let borrowed = Circle { radius: 3 };
    let shapes = [
        Shape::Boxed(Box::new(Circle { radius: 1 })),
        Shape::Shared(Rc::new(Circle { radius: 2 })),
        Shape::Borrowed(&borrowed),
        Shape::Plain(Circle { radius: 4 }),
    ];
    let descriptions: Vec<String> = shapes.iter().map(Shape::describe).collect();
    assert_eq!(
        descriptions,
        ["circle 1", "circle 2", "circle 3", "circle 4"]
    );
    let radii: Vec<u32> = shapes.iter().map(|shape| *shape.radius()).collect();
    assert_eq!(radii, [1, 2, 3, 4]);
    let radii: Vec<u32> = shapes.iter().map(Shape::radius_by_value).collect();
    assert_eq!(radii, [1, 2, 3, 4]);
    let circles: Vec<&Circle> = shapes.iter().map(Shape::circle).collect();
    assert_eq!(
        circles,
        [
            &Circle { radius: 1 },
            &Circle { radius: 2 },
            &Circle { radius: 3 },
            &Circle { radius: 4 },
        ]
    );
    // The `value` arm of a borrowed variant is the very circle it borrows.
    assert!(std::ptr::eq(shapes[2].circle(), &borrowed));
}

// A `Box` also lets a method borrow the target mutably or take it by value;
// `Held` reaches its circle through a field and then the box in it.
#[whichever::dispatch]
pub enum Owned {
    #[dispatch(deref)]
    Boxed(Box<Circle>),
    #[dispatch(via = .boxed*)]
    Held(Holder),
    Plain(Circle),
}

#[whichever::dispatch]
impl Owned {
    #[dispatch(*)]
    pub fn describe(&self) -> String;

    #[dispatch(*)]
    pub fn grow(&mut self, by: u32);

    #[dispatch(*)]
    pub fn into_radius(self) -> u32;

    #[dispatch(*, field = radius)]
    pub fn radius(&self) -> &u32;

    #[dispatch(*, field = radius)]
    pub fn radius_mut(&mut self) -> &mut u32;

    #[dispatch(*, value)]
    pub fn circle(&self) -> &Circle;
}

fn owned() -> Vec<Owned> {
    vec![
        Owned::Boxed(Box::new(Circle { radius: 1 })),
        Owned::Held(Holder {
            boxed: Box::new(Circle { radius: 2 }),
        }),
        Owned::Plain(Circle { radius: 3 }),
    ]
}

#[test]
fn every_receiver_reaches_through_a_box_and_a_route_ending_in_a_deref() {
    let mut shapes = owned();
    let descriptions: Vec<String> = shapes.iter().map(Owned::describe).collect();
    assert_eq!(descriptions, ["circle 1", "circle 2", "circle 3"]);
    for shape in &mut shapes {
        shape.grow(10);
    }
    let radii: Vec<u32> = shapes.iter().map(|shape| *shape.radius()).collect();
    assert_eq!(radii, [11, 12, 13]);
    for shape in &mut shapes {
        *shape.radius_mut() += 100;
    }
    let circles: Vec<&Circle> = shapes.iter().map(Owned::circle).collect();
    assert_eq!(
        circles,
        [
            &Circle { radius: 111 },
            &Circle { radius: 112 },
            &Circle { radius: 113 },
        ]
    );
    let radii: Vec<u32> = shapes.into_iter().map(Owned::into_radius).collect();
    assert_eq!(radii, [111, 112, 113]);
}

/// A payload whose inherent `as_ref` returns another value of its own type,
/// the type `Box<Draft>` and `Rc<Draft>` answer `AsRef::as_ref` with, so an
/// arm calling the pointer's method would compile and return the payload
/// itself instead.
#[derive(Debug, PartialEq)]
pub struct Draft(pub u32);

/// The draft every other one is revised from.
static ORIGINAL: Draft = Draft(0);

impl Draft {
    /// Returns the draft this one is revised from.
    pub fn as_ref(&self) -> &Draft {
        &ORIGINAL
    }
}

#[whichever::dispatch]
pub enum Document {
    #[dispatch(deref)]
    Boxed(Box<Draft>),
    #[dispatch(deref)]
    Shared(Rc<Draft>),
    Plain(Draft),
}

#[whichever::dispatch]
impl Document {
    #[dispatch(*)]
    pub fn as_ref(&self) -> &Draft;
}

#[test]
fn the_payload_answers_and_not_the_pointers_method_of_its_name() {
    let documents = [
        Document::Boxed(Box::new(Draft(1))),
        Document::Shared(Rc::new(Draft(2))),
        Document::Plain(Draft(3)),
    ];
    let answers: Vec<&Draft> = documents.iter().map(Document::as_ref).collect();
    assert_eq!(answers, [&Draft(0), &Draft(0), &Draft(0)]);
    assert!(answers
        .iter()
        .all(|answer| std::ptr::eq(*answer, &ORIGINAL)));
}

// A dispatch struct reaches its circles through a `Box` too: `*` is the
// field under `deref`, and `holder` is reached by the method's own route.
#[whichever::dispatch]
pub struct Labelled {
    #[dispatch(deref)]
    shape: Box<Circle>,
    holder: Holder,
    pub label: &'static str,
}

#[whichever::dispatch]
impl Labelled {
    #[dispatch(*)]
    pub fn describe(&self) -> String;

    #[dispatch(*)]
    pub fn grow(&mut self, by: u32);

    #[dispatch(*)]
    pub fn into_radius(self) -> u32;

    #[dispatch(*, field = radius)]
    pub fn radius(&self) -> &u32;

    #[dispatch(holder, via = .boxed*, call = describe)]
    pub fn held_description(&self) -> String;

    #[dispatch(holder, via = .boxed*, field = radius)]
    pub fn held_radius(&self) -> &u32;
}

#[test]
fn a_struct_reaches_its_field_through_a_box() {
    let mut labelled = Labelled {
        shape: Box::new(Circle { radius: 1 }),
        holder: Holder {
            boxed: Box::new(Circle { radius: 5 }),
        },
        label: "a",
    };
    assert_eq!(labelled.describe(), "circle 1");
    labelled.grow(2);
    assert_eq!(*labelled.radius(), 3);
    assert_eq!(labelled.held_description(), "circle 5");
    assert_eq!(*labelled.held_radius(), 5);
    assert_eq!(labelled.label, "a");
    assert_eq!(labelled.into_radius(), 3);
}
