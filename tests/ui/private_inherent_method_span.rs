//! rustc's "private method defined here" lands on the method, not on the
//! enum's attribute.
mod shapes {
    pub struct Circle(pub u32);

    impl Circle {
        pub fn radius(&self) -> u32 {
            self.0
        }
    }

    #[whichever::dispatch]
    pub enum AnyShape {
        Circle(Circle),
    }

    #[whichever::dispatch]
    impl AnyShape {
        #[dispatch(*)]
        fn radius(&self) -> u32;
    }
}

fn main() {
    let shape = shapes::AnyShape::Circle(shapes::Circle(1));
    let _ = shape.radius();
}
