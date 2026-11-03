//! `mismatch` is for a method with a `Self` argument; anywhere else it is a
//! mistake, reported where the trait is declared.
#[whichever::dispatchable]
trait Shape {
    #[dispatch(mismatch = 0)]
    fn area(&self) -> u32;
}

fn main() {}
