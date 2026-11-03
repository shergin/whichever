//! One rejected attribute in a stack leaves the others standing: the enum,
//! its conversions and the impls of the attributes that parsed are
//! emitted, so the one error stands alone.
#[whichever::dispatchable]
trait Area {
    fn area(&self) -> u32;
}

#[whichever::dispatchable]
trait Name {
    fn name(&self) -> &'static str;
}

#[whichever::dispatchable]
trait Sides {
    fn sides(&self) -> u32;
}

struct Sq;

impl Area for Sq {
    fn area(&self) -> u32 {
        4
    }
}

impl Name for Sq {
    fn name(&self) -> &'static str {
        "square"
    }
}

impl Sides for Sq {
    fn sides(&self) -> u32 {
        4
    }
}

#[whichever::dispatch(Area)]
#[whichever::dispatch(Name, from = 1)]
#[whichever::dispatch(Sides)]
enum Shape {
    Sq(Sq),
}

fn main() {
    let shape: Shape = Sq.into();
    let _ = shape.area() + shape.sides();
}
