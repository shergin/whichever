//! The `Self` return limit names the subject, a struct here, and the way
//! out of the path the method is on.
pub struct U(u64);

impl U {
    pub fn repeated(self, times: usize) -> Vec<Self> {
        (0..times).map(|_| U(self.0)).collect()
    }
}

#[whichever::dispatch]
pub struct A(U);

#[whichever::dispatch]
impl A {
    #[dispatch(*)]
    pub fn repeated(self, times: usize) -> Vec<Self>;
}

fn main() {}
