//! A second `#[dispatch]` on one inherent method is reported on itself;
//! rustc would only say the attribute cannot be found.
struct Greedy;
struct Beam;

impl Greedy {
    fn decode(&self) -> u32 {
        1
    }
}

impl Beam {
    fn decode(&self) -> Result<u32, ()> {
        Ok(2)
    }
}

#[whichever::dispatch]
enum Decoder {
    Greedy(Greedy),
    Beam(Beam),
}

#[whichever::dispatch]
impl Decoder {
    #[dispatch(Greedy, with = Ok)]
    #[dispatch(Beam)]
    fn decode(&self) -> Result<u32, ()> {
        Err(())
    }
}

fn main() {}
