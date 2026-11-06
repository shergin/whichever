#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

#[derive(Debug, PartialEq)]
struct Circle(u32);
#[derive(Debug, PartialEq)]
struct Square(u32);

impl Circle {
    fn radius(&self) -> u32 {
        self.0
    }
    fn describe(&self, prefix: &str) -> String {
        format!("{prefix} circle {}", self.0)
    }
    async fn fetch(&self) -> u32 {
        self.0
    }
    fn into_radius(self) -> u32 {
        self.0
    }
}

impl Square {
    fn describe(&self, prefix: &str) -> String {
        format!("{prefix} square {}", self.0)
    }
    async fn fetch(&self) -> u32 {
        self.0 * 10
    }
    fn grow(&mut self, by: u32) {
        self.0 += by;
    }
}

impl Shape for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
}

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(Shape, try_from = true)]
enum AnyShape {
    Circle(Circle),
    Square(Square),
}

#[whichever::dispatch]
impl AnyShape {
    /// Only circles have a radius.
    #[dispatch(Circle, with = Some)]
    pub fn radius(&self) -> Option<u32> {
        None
    }

    #[dispatch(*)]
    pub fn describe(&self, prefix: &str) -> String;

    #[dispatch(*, call = describe)]
    pub fn label(&self, prefix: &str) -> String;

    #[dispatch(*)]
    pub async fn fetch(&self) -> u32;

    /// Circles keep their size; the argument is still in scope here.
    #[dispatch(Square)]
    pub fn grow(&mut self, by: u32) {
        assert!(by > 0, "a circle cannot grow by {by}");
    }

    #[dispatch(Circle, with = Some)]
    pub fn into_radius(self) -> Option<u32> {
        None
    }

    pub fn untouched(&self) -> u8 {
        7
    }
}

#[test]
fn marked_methods_forward_and_the_body_is_the_fallback() {
    let circle: AnyShape = Circle(2).into();
    let mut square: AnyShape = Square(3).into();
    assert_eq!((circle.radius(), square.radius()), (Some(2), None));
    assert_eq!(circle.describe("a"), "a circle 2");
    assert_eq!(square.label("the"), "the square 3");
    assert_eq!(pollster::block_on(square.fetch()), 30);
    square.grow(1);
    assert_eq!(square, AnyShape::Square(Square(4)));
    let mut unchanged: AnyShape = Circle(5).into();
    unchanged.grow(1);
    assert_eq!(unchanged, AnyShape::Circle(Circle(5)));
    assert_eq!(unchanged.untouched(), 7);
    assert_eq!(circle.into_radius(), Some(2));
    assert_eq!(AnyShape::from(Square(1)).into_radius(), None);
}

#[test]
fn try_from_returns_the_payload_or_the_enum_back() {
    let circle: Result<Circle, AnyShape> = AnyShape::Circle(Circle(1)).try_into();
    assert_eq!(circle, Ok(Circle(1)));
    let not_a_square: Result<Square, AnyShape> = AnyShape::Circle(Circle(1)).try_into();
    assert_eq!(not_a_square, Err(AnyShape::Circle(Circle(1))));
}
