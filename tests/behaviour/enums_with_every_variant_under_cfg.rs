//! An enum whose every variant is under `#[cfg]` holds no variant at all in
//! the builds where none of them is compiled in. A reference to it still
//! counts as inhabited, so the `match` over a reference receiver looks at
//! the enum itself, `match *self { Variant(ref inner) => .. }`, as a hand
//! writes it, and that is exhaustive with no arms left.
//!
//! An integration test is compiled with `cfg(test)`, so `#[cfg(not(test))]`
//! is a `cfg` that is off here and `#[cfg(test)]` one that is on. The enums
//! under the first only have to compile; the ones under the second hold
//! values, and their tests call every method.

use std::pin::Pin;

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn grow(&mut self, by: u32);
    fn into_area(self) -> u32;
    fn name(&self) -> &str;
    fn side_mut(&mut self) -> &mut u32;
    fn poll_area(self: Pin<&mut Self>) -> u32;
    fn peek(self: Pin<&Self>) -> u32;
    async fn measured(&mut self) -> u32;
    #[dispatch(mismatch = false)]
    fn same_as(&self, other: &Self) -> bool;
}

#[derive(Debug, PartialEq)]
pub struct Square {
    pub side: u32,
    pub name: String,
}

impl Square {
    pub fn new(side: u32, name: &str) -> Self {
        Square {
            side,
            name: name.to_owned(),
        }
    }

    pub fn side(&self) -> u32 {
        self.side
    }

    pub fn reset(&mut self) {
        self.side = 0;
    }
}

impl Shape for Square {
    fn area(&self) -> u32 {
        self.side * self.side
    }
    fn grow(&mut self, by: u32) {
        self.side += by;
    }
    fn into_area(self) -> u32 {
        self.area()
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn side_mut(&mut self) -> &mut u32 {
        &mut self.side
    }
    fn poll_area(self: Pin<&mut Self>) -> u32 {
        self.get_mut().area()
    }
    fn peek(self: Pin<&Self>) -> u32 {
        self.side
    }
    async fn measured(&mut self) -> u32 {
        self.side += 1;
        self.side
    }
    fn same_as(&self, other: &Self) -> bool {
        self.side == other.side
    }
}

/// A `Self` return, rewrapped in the variant, which a variant with other
/// fields beside its payload cannot be.
#[whichever::dispatchable]
pub trait Rescale {
    fn doubled(&self) -> Self
    where
        Self: Sized;
}

impl Rescale for Square {
    fn doubled(&self) -> Self {
        Square::new(self.side * 2, &self.name)
    }
}

/// A module whose shape is reached through a field, the variant's route.
pub struct Module {
    pub base: Square,
}

/// The only variant is compiled out, so the enum is empty in this build.
/// It has no value to call anything on: it only has to compile.
#[whichever::dispatch(Shape)]
#[whichever::dispatch(Rescale)]
pub enum Lone {
    #[cfg(not(test))]
    Square(Square),
}

/// Two variants, both compiled out.
#[whichever::dispatch(Shape, from = false)]
pub enum Pair {
    #[cfg(not(test))]
    First(Square),
    #[cfg(not(test))]
    Second(Square),
}

#[whichever::dispatch]
impl Pair {
    #[dispatch(*)]
    pub fn side(&self) -> u32;

    #[dispatch(*, call = area)]
    pub fn size(&self) -> u32;

    #[dispatch(*)]
    pub fn reset(&mut self);
}

/// Every variant is compiled in: the same `match *self` runs.
#[whichever::dispatch(Shape, from = false)]
#[whichever::dispatch(Rescale, from = false)]
#[derive(Debug, PartialEq)]
pub enum Present {
    #[cfg(test)]
    Square(Square),
    #[cfg(test)]
    Other(Square),
}

#[whichever::dispatch]
impl Present {
    #[dispatch(*)]
    pub fn side(&self) -> u32;

    #[dispatch(*, call = area)]
    pub fn size(&self) -> u32;

    #[dispatch(*)]
    pub fn reset(&mut self);

    /// A field of the payload, borrowed as the return type asks.
    #[dispatch(*, field = side)]
    pub fn side_ref(&self) -> &u32;
}

/// A payload among other fields, marked in a tuple and in a struct-like
/// variant, and one reached through a route.
#[whichever::dispatch(Shape, from = false)]
pub enum Marked {
    #[cfg(test)]
    Tagged(u8, #[dispatch] Square),
    #[cfg(test)]
    Row {
        id: u32,
        #[dispatch]
        cell: Square,
    },
    #[cfg(test)]
    #[dispatch(via = .base)]
    Module(Module),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Handle(pub u64);

/// A holder whose handle is reached through a field, the variant's route.
pub struct Holder {
    pub handle: Handle,
}

/// A plain payload and a marked field of a struct-like variant beside a
/// routed one, each read by value through the binding the `match *self`
/// makes, `ref inner`.
#[whichever::dispatch(from = false)]
pub enum Handled {
    #[cfg(test)]
    Row {
        id: u32,
        #[dispatch]
        handle: Handle,
    },
    #[cfg(test)]
    #[dispatch(via = .handle)]
    Held(Holder),
    #[cfg(test)]
    Bare(Handle),
}

#[whichever::dispatch]
impl Handled {
    #[dispatch(*, value)]
    pub fn handle(&self) -> Handle;
}

#[whichever::dispatchable]
pub trait Measure {
    fn surface(&self) -> u32;
    fn stretch(&mut self, by: u32);
    fn label(&self) -> &str;
    fn edge_mut(&mut self) -> &mut u32;
}

impl Measure for Square {
    fn surface(&self) -> u32 {
        Shape::area(self)
    }
    fn stretch(&mut self, by: u32) {
        Shape::grow(self, by);
    }
    fn label(&self) -> &str {
        Shape::name(self)
    }
    fn edge_mut(&mut self) -> &mut u32 {
        Shape::side_mut(self)
    }
}

/// Dereferenced payloads, a box of a sized one and a trait object, off.
#[whichever::dispatch(Measure, from = false)]
pub enum BoxedOff {
    #[cfg(not(test))]
    #[dispatch(deref)]
    Heap(Box<Square>),
    #[cfg(not(test))]
    #[dispatch(deref)]
    Dyn(Box<dyn Measure>),
}

/// The same, on.
#[whichever::dispatch(Measure, from = false)]
pub enum Boxed {
    #[cfg(test)]
    #[dispatch(deref)]
    Heap(Box<Square>),
    #[cfg(test)]
    #[dispatch(deref)]
    Dyn(Box<dyn Measure>),
}

#[whichever::dispatchable]
pub trait Priced {
    fn price(&self) -> u32;
    fn discount(&mut self, by: u32);
}

impl Priced for Square {
    fn price(&self) -> u32 {
        self.side * 10
    }
    fn discount(&mut self, by: u32) {
        self.side -= by;
    }
}

/// A partial block over an enum whose every variant is off: the marks name
/// a variant, and the bodies answer for the rest.
#[whichever::dispatch]
pub enum StockOff {
    #[cfg(not(test))]
    Square(Square),
    #[cfg(not(test))]
    Gift,
}

#[whichever::dispatch]
impl Priced for StockOff {
    #[dispatch(Square)]
    fn price(&self) -> u32 {
        0
    }
    #[dispatch(Square)]
    fn discount(&mut self, by: u32) {
        let _ = by;
    }
}

/// The same, on.
#[whichever::dispatch]
pub enum Stock {
    #[cfg(test)]
    Square(Square),
    #[cfg(test)]
    Gift,
}

#[whichever::dispatch]
impl Priced for Stock {
    /// A gift is free.
    #[dispatch(Square)]
    fn price(&self) -> u32 {
        0
    }
    /// A gift has no price to lower.
    #[dispatch(Square)]
    fn discount(&mut self, by: u32) {
        let _ = by;
    }
}

fn implements_shape<T: Shape>() {}
fn implements_measure<T: Measure + ?Sized>() {}
fn implements_priced<T: Priced>() {}

#[test]
fn enums_with_every_variant_off_compile_and_implement_the_traits() {
    implements_shape::<Lone>();
    implements_shape::<Pair>();
    implements_measure::<BoxedOff>();
    implements_priced::<StockOff>();
    // Naming each method instantiates its body, so that a body which does
    // not compile in this build cannot hide.
    let _: fn(&Lone) -> u32 = <Lone as Shape>::area;
    let _: fn(&mut Lone, u32) = <Lone as Shape>::grow;
    let _: fn(Lone) -> u32 = <Lone as Shape>::into_area;
    let _: fn(&Lone) -> &str = <Lone as Shape>::name;
    let _: fn(&mut Lone) -> &mut u32 = <Lone as Shape>::side_mut;
    let _: fn(Pin<&mut Lone>) -> u32 = <Lone as Shape>::poll_area;
    let _: fn(Pin<&Lone>) -> u32 = <Lone as Shape>::peek;
    let _: fn(&Lone) -> Lone = <Lone as Rescale>::doubled;
    let _: fn(&Lone, &Lone) -> bool = <Lone as Shape>::same_as;
    let _ = <Lone as Shape>::measured;
    let _: fn(&Pair) -> u32 = <Pair as Shape>::area;
    let _: fn(&mut Pair) -> &mut u32 = <Pair as Shape>::side_mut;
    let _: fn(&Pair) -> u32 = Pair::side;
    let _: fn(&Pair) -> u32 = Pair::size;
    let _: fn(&mut Pair) = Pair::reset;
    let _: fn(&BoxedOff) -> &str = <BoxedOff as Measure>::label;
    let _: fn(&mut BoxedOff) -> &mut u32 = <BoxedOff as Measure>::edge_mut;
    let _: fn(&StockOff) -> u32 = <StockOff as Priced>::price;
    let _: fn(&mut StockOff, u32) = <StockOff as Priced>::discount;
}

#[test]
fn shared_and_exclusive_receivers_reach_the_payload() {
    for mut shape in [
        Present::Square(Square::new(3, "square")),
        Present::Other(Square::new(3, "other")),
    ] {
        assert_eq!(shape.area(), 9);
        shape.grow(1);
        assert_eq!(shape.area(), 16);
        *shape.side_mut() = 5;
        assert_eq!(shape.side(), 5);
        assert_eq!(shape.size(), 25);
        shape.reset();
        assert_eq!(shape.side(), 0);
        shape.grow(2);
        assert_eq!(shape.into_area(), 4);
    }
}

#[test]
fn returned_references_borrow_from_the_payload() {
    let mut shape = Present::Square(Square::new(3, "square"));
    let name: *const u8 = shape.name().as_ptr();
    let side: *const u32 = shape.side_ref();
    let Present::Square(square) = &shape else {
        unreachable!()
    };
    assert_eq!(name, square.name.as_ptr());
    assert_eq!(side, &square.side as *const u32);
    assert_eq!(shape.name(), "square");
    let side: *mut u32 = shape.side_mut();
    let Present::Square(square) = &mut shape else {
        unreachable!()
    };
    assert_eq!(side, &mut square.side as *mut u32);
}

#[test]
fn pinned_receivers_reach_the_payload() {
    let mut shape = Present::Other(Square::new(4, "other"));
    assert_eq!(Pin::new(&mut shape).poll_area(), 16);
    assert_eq!(Pin::new(&shape).peek(), 4);
}

#[test]
fn a_self_return_is_rewrapped() {
    let shape = Present::Other(Square::new(4, "other"));
    assert_eq!(shape.doubled(), Present::Other(Square::new(8, "other")));
}

#[test]
fn an_async_method_is_awaited() {
    let mut shape = Present::Square(Square::new(1, "square"));
    assert_eq!(pollster::block_on(shape.measured()), 2);
    assert_eq!(pollster::block_on(shape.measured()), 3);
    assert_eq!(shape.side(), 3);
}

#[test]
fn a_self_argument_meets_the_same_variant_and_mismatches_another() {
    let a = Present::Square(Square::new(2, "a"));
    let b = Present::Square(Square::new(2, "b"));
    let c = Present::Other(Square::new(2, "c"));
    assert!(a.same_as(&b));
    assert!(!a.same_as(&c));
}

#[test]
fn marked_fields_and_routes_are_the_target() {
    let mut shapes = [
        Marked::Tagged(7, Square::new(2, "tagged")),
        Marked::Row {
            id: 1,
            cell: Square::new(2, "row"),
        },
        Marked::Module(Module {
            base: Square::new(2, "module"),
        }),
    ];
    for shape in &mut shapes {
        assert_eq!(shape.area(), 4);
        shape.grow(1);
        *shape.side_mut() += 1;
        assert_eq!(Pin::new(&*shape).peek(), 4);
        assert_eq!(Pin::new(&mut *shape).poll_area(), 16);
    }
    assert_eq!(shapes[0].name(), "tagged");
    assert_eq!(shapes[1].name(), "row");
    assert_eq!(shapes[2].name(), "module");
    assert!(matches!(
        shapes[0],
        Marked::Tagged(7, Square { side: 4, .. })
    ));
    assert!(matches!(
        shapes[1],
        Marked::Row {
            id: 1,
            cell: Square { side: 4, .. }
        }
    ));
}

#[test]
fn a_payload_or_a_marked_field_is_read_by_value_through_the_binding() {
    let row = Handled::Row {
        id: 1,
        handle: Handle(4),
    };
    let held = Handled::Held(Holder { handle: Handle(5) });
    assert_eq!(row.handle(), Handle(4));
    assert_eq!(held.handle(), Handle(5));
    let bare = Handled::Bare(Handle(6));
    assert_eq!(bare.handle(), Handle(6));
}

#[test]
fn dereferenced_payloads_are_reached_through_the_box() {
    let mut shapes = [
        Boxed::Heap(Box::new(Square::new(3, "heap"))),
        Boxed::Dyn(Box::new(Square::new(3, "dyn"))),
    ];
    for shape in &mut shapes {
        assert_eq!(shape.surface(), 9);
        shape.stretch(1);
        *shape.edge_mut() += 1;
        assert_eq!(shape.surface(), 25);
    }
    assert_eq!(shapes[0].label(), "heap");
    assert_eq!(shapes[1].label(), "dyn");
}

#[test]
fn a_partial_block_forwards_the_named_variant_and_answers_for_the_rest() {
    let mut square = Stock::Square(Square::new(3, "square"));
    assert_eq!(square.price(), 30);
    square.discount(1);
    assert_eq!(square.price(), 20);
    let mut gift = Stock::Gift;
    gift.discount(1);
    assert_eq!(gift.price(), 0);
}
