//! The program behind `docs/tutorial.md`: `cargo run --example tour`.
//!
//! Each module is one step of the tour, in the order the page takes them,
//! and each `run` checks what its step claims. The code the page shows is
//! the code here.

/// Step 1: a trait, two types that implement it, and an enum over them.
mod first_enum {
    #[whichever::dispatchable]
    pub trait Shape {
        fn area(&self) -> u32;
        fn name(&self) -> &'static str {
            "shape"
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Circle(pub u32);

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Square(pub u32);

    impl Shape for Circle {
        fn area(&self) -> u32 {
            3 * self.0 * self.0
        }
        fn name(&self) -> &'static str {
            "circle"
        }
    }

    // `Square` keeps the trait's default `name`.
    impl Shape for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
    }

    #[whichever::dispatch(Shape)]
    pub enum AnyShape {
        Circle(Circle),
        Square(Square),
    }

    pub fn run() {
        let shapes: Vec<AnyShape> = vec![Circle(1).into(), Square(2).into()];
        let total: u32 = shapes.iter().map(|shape| shape.area()).sum();
        assert_eq!(total, 7);
        println!("1. first enum: total area {total}");
    }
}

/// Step 2: two modules stand in for two crates. `shapes_crate` owns the
/// trait, and `app_crate` owns the enum and sees the trait only through
/// its imports, as another crate would.
mod across_crates {
    mod shapes_crate {
        #[derive(Debug, PartialEq)]
        pub struct Size {
            pub width: u32,
            pub height: u32,
        }

        #[whichever::dispatchable]
        pub trait Measure {
            fn size(&self) -> Size;
        }
    }

    mod app_crate {
        // `Measure` brings its carrier macro along. `Size` is imported
        // because the generated impl writes it as the trait spells it.
        use super::shapes_crate::{Measure, Size};
        use crate::first_enum::{Circle, Square};

        impl Measure for Circle {
            fn size(&self) -> Size {
                Size {
                    width: 2 * self.0,
                    height: 2 * self.0,
                }
            }
        }

        impl Measure for Square {
            fn size(&self) -> Size {
                Size {
                    width: self.0,
                    height: self.0,
                }
            }
        }

        #[whichever::dispatch(Measure)]
        pub enum Measured {
            Circle(Circle),
            Square(Square),
        }

        pub fn run() {
            let circle = Measured::from(Circle(3));
            assert_eq!(
                circle.size(),
                Size {
                    width: 6,
                    height: 6
                }
            );
            println!("2. across crates: a circle of radius 3 is 6 wide");
        }
    }

    pub fn run() {
        app_crate::run();
    }
}

/// Step 3: a provided method is forwarded like any other, so the payload's
/// own override is what runs.
mod provided_methods {
    use crate::first_enum::{AnyShape, Circle, Shape, Square};

    pub fn run() {
        let circle = AnyShape::from(Circle(1));
        let square = AnyShape::from(Square(1));
        assert_eq!(circle.name(), "circle");
        assert_eq!(square.name(), "shape");
        println!(
            "3. provided methods: {} and {}",
            circle.name(),
            square.name()
        );
    }
}

/// Step 4: `Self` in a return type and in an argument.
mod self_in_signatures {
    use crate::first_enum::{Circle, Shape, Square};

    #[whichever::dispatchable]
    pub trait Resize {
        fn scaled(&self, factor: u32) -> Self;

        /// Two different kinds of shape are never congruent.
        #[dispatch(mismatch = false)]
        fn congruent(&self, other: &Self) -> bool;
    }

    impl Resize for Circle {
        fn scaled(&self, factor: u32) -> Self {
            Circle(self.0 * factor)
        }
        fn congruent(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    impl Resize for Square {
        fn scaled(&self, factor: u32) -> Self {
            Square(self.0 * factor)
        }
        fn congruent(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    #[whichever::dispatch(Shape)]
    #[whichever::dispatch(Resize)]
    pub enum AnyShape {
        Circle(Circle),
        Square(Square),
    }

    pub fn run() {
        let square = AnyShape::from(Square(2));
        let bigger = square.scaled(3);
        assert!(matches!(bigger, AnyShape::Square(Square(6))));
        assert_eq!(bigger.area(), 36);

        let circle = AnyShape::from(Circle(2));
        assert!(square.congruent(&AnyShape::from(Square(2))));
        assert!(!square.congruent(&circle));
        println!("4. Self in signatures: a square scaled by 3 has area 36");
    }
}

/// Step 5: associated types and consts. The tour switches to storage
/// backends here, since shapes have no natural associated type.
mod associated_items {
    use std::collections::HashMap;
    use std::convert::Infallible;

    #[whichever::dispatchable]
    pub trait Storage {
        const BLOCK_SIZE: usize;
        type Error;
        fn load(&self, key: &str) -> Result<Vec<u8>, Self::Error>;
    }

    pub struct Memory(pub HashMap<String, Vec<u8>>);
    pub struct Network;

    #[derive(Debug, PartialEq)]
    pub struct Timeout;

    impl Storage for Memory {
        const BLOCK_SIZE: usize = 4096;
        type Error = Infallible;
        fn load(&self, key: &str) -> Result<Vec<u8>, Infallible> {
            Ok(self.0.get(key).cloned().unwrap_or_default())
        }
    }

    impl Storage for Network {
        const BLOCK_SIZE: usize = 4096;
        type Error = Timeout;
        fn load(&self, _key: &str) -> Result<Vec<u8>, Timeout> {
            Err(Timeout)
        }
    }

    /// The one error type the enum returns, whichever backend failed.
    #[derive(Debug, PartialEq)]
    pub enum StorageError {
        Timeout,
    }

    impl From<Infallible> for StorageError {
        fn from(never: Infallible) -> Self {
            match never {}
        }
    }

    impl From<Timeout> for StorageError {
        fn from(_: Timeout) -> Self {
            StorageError::Timeout
        }
    }

    #[whichever::dispatch(Storage, Error = StorageError)]
    pub enum AnyStorage {
        Memory(Memory),   // Error = Infallible
        Network(Network), // Error = Timeout
    }

    pub fn run() {
        assert_eq!(AnyStorage::BLOCK_SIZE, 4096);

        let mut values = HashMap::new();
        values.insert("greeting".to_string(), b"hello".to_vec());
        let memory = AnyStorage::from(Memory(values));
        assert_eq!(memory.load("greeting"), Ok(b"hello".to_vec()));

        let network = AnyStorage::from(Network);
        assert_eq!(network.load("greeting"), Err(StorageError::Timeout));
        println!("5. associated items: blocks of 4096, errors as `StorageError`");
    }
}

/// Step 6: conversions between the enum and its payloads.
mod conversions {
    use crate::first_enum::{Circle, Shape, Square};

    #[derive(Debug)]
    #[whichever::dispatch(Shape, try_from = true)]
    pub enum AnyShape {
        Circle(Circle),
        Square(Square),
    }

    pub fn run() {
        let shape: AnyShape = Circle(2).into();
        assert_eq!(Circle::try_from(shape).unwrap(), Circle(2));

        // A failed `try_from` hands the enum back as the error.
        let shape: AnyShape = Square(2).into();
        let back = Circle::try_from(shape).unwrap_err();
        assert!(matches!(back, AnyShape::Square(Square(2))));
        println!("6. conversions: a square is not a circle, and comes back whole");
    }
}

/// Step 7: payloads behind a pointer, and one variant for any other shape.
mod behind_a_pointer {
    use crate::first_enum::{Circle, Shape, Square};
    use std::rc::Rc;

    #[whichever::dispatch(Shape)]
    pub enum Held<'a> {
        Circle(Circle),
        #[dispatch(deref)]
        Boxed(Box<Square>),
        #[dispatch(deref)]
        Shared(Rc<Square>),
        #[dispatch(deref)]
        Borrowed(&'a Square),
    }

    #[whichever::dispatch(Shape)]
    pub enum OpenShape {
        Circle(Circle),
        Square(Square),
        #[dispatch(deref)]
        Other(Box<dyn Shape>),
    }

    /// A shape the enum does not list.
    pub struct Triangle(pub u32);

    impl Shape for Triangle {
        fn area(&self) -> u32 {
            self.0 * self.0 / 2
        }
        fn name(&self) -> &'static str {
            "triangle"
        }
    }

    pub fn run() {
        let square = Square(3);
        let held = [
            Held::from(Box::new(Square(2))),
            Held::from(Rc::new(Square(2))),
            Held::from(&square),
        ];
        let areas: Vec<u32> = held.iter().map(|shape| shape.area()).collect();
        assert_eq!(areas, [4, 4, 9]);

        let other = OpenShape::Other(Box::new(Triangle(4)));
        assert_eq!((other.name(), other.area()), ("triangle", 8));
        println!("7. behind a pointer: a boxed triangle of area 8");
    }
}

/// Step 8: methods of the enum's own, forwarded to some variants or to all.
mod inherent_methods {
    use crate::first_enum::{AnyShape, Circle, Square};

    impl Circle {
        pub fn radius(&self) -> u32 {
            self.0
        }
        pub fn describe(&self) -> String {
            format!("circle of radius {}", self.0)
        }
    }

    impl Square {
        pub fn describe(&self) -> String {
            format!("square of side {}", self.0)
        }
    }

    #[whichever::dispatch]
    impl AnyShape {
        /// Only circles have a radius.
        #[dispatch(Circle, with = Some)]
        pub fn radius(&self) -> Option<u32> {
            None
        }

        #[dispatch(*)]
        pub fn describe(&self) -> String;
    }

    pub fn run() {
        let circle = AnyShape::from(Circle(2));
        let square = AnyShape::from(Square(3));
        assert_eq!(circle.radius(), Some(2));
        assert_eq!(square.radius(), None);
        assert_eq!(square.describe(), "square of side 3");
        println!("8. inherent methods: {}", circle.describe());
    }
}

/// Step 9: a partial impl block writes some of a trait's methods, and the
/// rest are forwarded.
mod partial_impl_block {
    use crate::first_enum::{AnyShape, Circle, Square};

    #[whichever::dispatchable]
    pub trait Sides {
        /// The smallest shape of this kind.
        fn unit() -> Self;
        fn sides(&self) -> u32;
        fn summary(&self) -> String {
            format!("{} sides", self.sides())
        }
    }

    impl Sides for Circle {
        fn unit() -> Self {
            Circle(1)
        }
        fn sides(&self) -> u32 {
            0
        }
    }

    impl Sides for Square {
        fn unit() -> Self {
            Square(1)
        }
        fn sides(&self) -> u32 {
            4
        }
    }

    #[whichever::dispatch]
    impl Sides for AnyShape {
        fn unit() -> Self {
            AnyShape::Square(Square::unit())
        }
        fn summary(&self) -> String {
            format!("a shape with {} sides", self.sides())
        }
    }

    pub fn run() {
        let unit = AnyShape::unit();
        assert!(matches!(unit, AnyShape::Square(Square(1))));
        assert_eq!(unit.sides(), 4);
        assert_eq!(unit.summary(), "a shape with 4 sides");
        println!("9. partial impl block: {}", unit.summary());
    }
}

/// Step 10: a trait from `std`, through the restatements that ship in
/// `whichever::foreign`.
mod foreign_traits {
    use std::fmt;
    use std::num::ParseIntError;

    #[derive(Debug)]
    pub struct TooBig(pub u32);

    impl fmt::Display for TooBig {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} is too big", self.0)
        }
    }

    #[derive(Debug)]
    #[whichever::dispatch(whichever::foreign::Display)]
    pub enum ShapeError {
        Parse(ParseIntError),
        TooBig(TooBig),
        #[dispatch(from = false)]
        Unknown(String),
    }

    pub fn run() {
        let parse = ShapeError::from("x".parse::<u32>().unwrap_err());
        let too_big = ShapeError::from(TooBig(1000));
        let unknown = ShapeError::Unknown("hexagon".to_string());
        assert_eq!(parse.to_string(), "invalid digit found in string");
        assert_eq!(too_big.to_string(), "1000 is too big");
        assert_eq!(unknown.to_string(), "hexagon");
        println!("10. foreign traits: \"{too_big}\"");
    }
}

/// Step 11: a public struct around a private dispatch enum.
mod structs {
    use crate::first_enum::{AnyShape, Shape, Square};

    #[whichever::dispatch(Shape)]
    pub struct Labelled {
        #[dispatch]
        shape: AnyShape,
        label: &'static str,
    }

    pub fn run() {
        let labelled = Labelled {
            shape: Square(3).into(),
            label: "tile",
        };
        assert_eq!((labelled.area(), labelled.name()), (9, "shape"));
        println!(
            "11. structs: the {} has area {}",
            labelled.label,
            labelled.area()
        );
    }
}

/// Step 12: `async fn` in a trait. `pollster` runs the futures to the end.
mod async_methods {
    use crate::first_enum::{Circle, Square};

    #[whichever::dispatchable]
    trait Render {
        async fn render(&self) -> String;
    }

    impl Render for Circle {
        async fn render(&self) -> String {
            format!("<circle r={}/>", self.0)
        }
    }

    impl Render for Square {
        async fn render(&self) -> String {
            format!("<rect width={}/>", self.0)
        }
    }

    #[whichever::dispatch(Render)]
    enum Drawing {
        Circle(Circle),
        Square(Square),
    }

    pub fn run() {
        let drawing = Drawing::from(Circle(2));
        let svg = pollster::block_on(drawing.render());
        assert_eq!(svg, "<circle r=2/>");
        println!("12. async methods: {svg}");
    }
}

/// Step 13: the fix for a limit. A method returning `impl Iterator` cannot
/// be forwarded by a `match`, so the trait returns a concrete type.
mod limits {
    use crate::first_enum::{Circle, Square};

    #[whichever::dispatchable]
    pub trait Edges {
        fn edges(&self) -> Vec<u32>;
    }

    impl Edges for Circle {
        fn edges(&self) -> Vec<u32> {
            Vec::new()
        }
    }

    impl Edges for Square {
        fn edges(&self) -> Vec<u32> {
            vec![self.0; 4]
        }
    }

    #[whichever::dispatch(Edges)]
    pub enum AnyShape {
        Circle(Circle),
        Square(Square),
    }

    pub fn run() {
        let square = AnyShape::from(Square(2));
        assert_eq!(square.edges(), [2, 2, 2, 2]);
        assert!(AnyShape::from(Circle(1)).edges().is_empty());
        println!("13. limits: a square has {} edges", square.edges().len());
    }
}

fn main() {
    first_enum::run();
    across_crates::run();
    provided_methods::run();
    self_in_signatures::run();
    associated_items::run();
    conversions::run();
    behind_a_pointer::run();
    inherent_methods::run();
    partial_impl_block::run();
    foreign_traits::run();
    structs::run();
    async_methods::run();
    limits::run();
}
