//! `#[dispatch]` goes on an enum, a struct, or an impl block of one.
#[whichever::dispatch]
fn area() -> u32 {
    1
}

fn main() {}
