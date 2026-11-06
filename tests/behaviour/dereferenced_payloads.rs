use std::rc::Rc;

#[whichever::dispatchable]
trait Area {
    fn area(&self) -> u32;
    #[dispatch(mismatch = false)]
    fn same_area(&self, other: &Self) -> bool;
}

#[whichever::dispatchable]
trait Grow {
    fn grow(&mut self, by: u32);
}

#[whichever::dispatchable]
trait Consume {
    fn into_side(self) -> u32;
}

struct Square(u32);
struct Circle(u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
    fn same_area(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Area for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
    fn same_area(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Grow for Square {
    fn grow(&mut self, by: u32) {
        self.0 += by;
    }
}

impl Consume for Square {
    fn into_side(self) -> u32 {
        self.0
    }
}

#[whichever::dispatch(Area)]
enum Shape<'a> {
    #[dispatch(deref)]
    Boxed(Box<Square>),
    #[dispatch(deref)]
    Shared(Rc<Circle>),
    #[dispatch(deref)]
    Borrowed(&'a Square),
    Plain(Circle),
}

#[whichever::dispatch(Grow)]
enum Growable<'a> {
    #[dispatch(deref)]
    Boxed(Box<Square>),
    #[whichever::dispatch(deref)]
    Borrowed(&'a mut Square),
}

#[whichever::dispatch(Consume)]
enum Consumable {
    #[dispatch(deref)]
    Boxed(Box<Square>),
    Plain(Square),
}

#[test]
fn the_trait_is_called_on_the_dereferenced_payload() {
    let square = Square(2);
    let shapes = [
        Shape::Boxed(Box::new(Square(2))),
        Shape::Shared(Rc::new(Circle(2))),
        Shape::Borrowed(&square),
        Shape::Plain(Circle(2)),
    ];
    let areas: Vec<u32> = shapes.iter().map(Area::area).collect();
    assert_eq!(areas, [4, 12, 4, 12]);
    assert!(shapes[0].same_area(&Shape::Boxed(Box::new(Square(2)))));
    assert!(!shapes[0].same_area(&shapes[2]));

    let mut boxed = Growable::Boxed(Box::new(Square(1)));
    boxed.grow(2);
    let mut owned = Square(5);
    let mut borrowed = Growable::Borrowed(&mut owned);
    borrowed.grow(1);
    assert_eq!(owned.0, 6);

    assert_eq!(Consumable::Boxed(Box::new(Square(3))).into_side(), 3);
    assert_eq!(Consumable::Plain(Square(4)).into_side(), 4);
}
