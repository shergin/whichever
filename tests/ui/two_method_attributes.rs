//! One `#[dispatch]` attribute per trait method.
#[whichever::dispatchable]
trait Shape {
    #[dispatch(mismatch = false)]
    #[dispatch(mismatch = true)]
    fn same(&self, other: &Self) -> bool;
}

fn main() {}
