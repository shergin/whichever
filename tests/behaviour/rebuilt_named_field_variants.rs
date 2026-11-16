//! A variant whose one field is named and marked is rebuilt by its braces
//! for a method returning `Self`, as a struct with one named field is, and
//! the variant beside it is rebuilt by its name.
#[whichever::dispatchable]
trait Shape {
    fn radius(&self) -> u32;
    fn doubled(self) -> Self;
}

#[derive(Debug, PartialEq)]
struct Circle(u32);

impl Shape for Circle {
    fn radius(&self) -> u32 {
        self.0
    }
    fn doubled(self) -> Self {
        Circle(self.0 * 2)
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(Shape)]
enum Any {
    Named {
        #[dispatch]
        inner: Circle,
    },
    Plain(Circle),
}

#[test]
fn the_named_variant_is_rebuilt_as_the_named_variant() {
    let named = Any::Named { inner: Circle(2) };
    assert_eq!(named.radius(), 2);
    assert_eq!(named.doubled(), Any::Named { inner: Circle(4) });
}

#[test]
fn the_variant_beside_it_is_rebuilt_by_its_name() {
    let plain = Any::from(Circle(3));
    assert_eq!(
        plain,
        Any::Plain(Circle(3)),
        "only the tuple variant converts"
    );
    assert_eq!(plain.radius(), 3);
    assert_eq!(plain.doubled(), Any::Plain(Circle(6)));
}
