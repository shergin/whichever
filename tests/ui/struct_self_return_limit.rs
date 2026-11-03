//! The `Self` return limit names the subject, a struct here, and the way
//! out of the path the method is on.
pub struct U(u64);

impl U {
    pub fn mul_u64(self, b: u64) -> (Self, bool) {
        (U(self.0 * b), false)
    }
}

#[whichever::dispatch]
pub struct A(U);

#[whichever::dispatch]
impl A {
    #[dispatch(*)]
    pub fn mul_u64(self, b: u64) -> (Self, bool);
}

fn main() {}
