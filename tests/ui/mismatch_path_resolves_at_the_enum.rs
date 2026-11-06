//! A path in a `mismatch` expression written on the trait method resolves
//! where the enum is, as the signature types do; a helper beside the trait
//! has to be spelled from `crate::`.
mod shapes {
    pub fn never() -> bool {
        false
    }

    #[whichever::dispatchable]
    pub trait Shape {
        fn area(&self) -> u32;
        #[dispatch(mismatch = never())]
        fn same(&self, other: &Self) -> bool;
    }
}

mod any {
    use crate::shapes::Shape;

    pub struct Square(pub u32);
    pub struct Circle(pub u32);

    impl Shape for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
        fn same(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    impl Shape for Circle {
        fn area(&self) -> u32 {
            3 * self.0 * self.0
        }
        fn same(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    #[whichever::dispatch(Shape)]
    pub enum Any {
        Square(Square),
        Circle(Circle),
    }
}

fn main() {}
