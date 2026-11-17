//! A restated struct gets no conversions, as no struct does, and says so in
//! the struct's words.
pub mod other {
    pub struct Square(pub u32);
    pub struct Solo(pub Square);
}

#[whichever::dispatch(remote = other::Solo, from = true)]
pub struct Solo(other::Square);

fn main() {}
