//! Methods a partial block marks with `#[dispatch(..)]` under `#[cfg]`: a
//! mark is the block's where its `cfg` holds, and the method is forwarded to
//! every variant where it does not. `cargo expand` evaluates the `cfg`, and
//! `test` is off there, so `name` and `kind`, marked under `not(test)`, show
//! the build where the `cfg` holds: `name` is left to the default and `kind`
//! takes its body for `Square`. `label` and `size`, marked under `test`,
//! show the build where it does not: both are forwarded to every variant.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str {
        "shape"
    }
    fn label(&self) -> &'static str {
        "shape"
    }
    fn kind(&self) -> &'static str {
        "shape"
    }
    fn size(&self) -> &'static str {
        "shape"
    }
}

pub struct Circle;

impl Shape for Circle {
    fn area(&self) -> u32 {
        3
    }
}

pub struct Square;

impl Shape for Square {
    fn area(&self) -> u32 {
        4
    }
}

#[whichever::dispatch]
pub enum AnyShape {
    Circle(Circle),
    Square(Square),
}

#[whichever::dispatch]
impl Shape for AnyShape {
    #[cfg(not(test))]
    #[dispatch(default)]
    fn name(&self) -> &'static str;

    #[cfg(test)]
    #[dispatch(default)]
    fn label(&self) -> &'static str;

    #[cfg(not(test))]
    #[dispatch(Circle)]
    fn kind(&self) -> &'static str {
        "body"
    }

    #[cfg(test)]
    #[dispatch(Circle)]
    fn size(&self) -> &'static str {
        "body"
    }
}

fn main() {}
