//! The one option of a trait method is `mismatch`.
#[whichever::dispatchable]
trait Shape {
    #[dispatch(fallback = false)]
    fn congruent(&self, other: &Self) -> bool;
}

fn main() {}
