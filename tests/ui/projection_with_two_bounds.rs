//! A projection through a trait parameter with two bounds cannot be
//! qualified when the parameter is given a concrete type; the trait names
//! the bound instead.
trait Format {
    type Value;
}

#[derive(Clone)]
struct Json;

impl Format for Json {
    type Value = String;
}

#[whichever::dispatchable]
trait Encoder<F: Format + Clone> {
    fn encode(&self, value: F::Value) -> usize;
}

struct Compact;

impl Encoder<Json> for Compact {
    fn encode(&self, value: String) -> usize {
        value.len()
    }
}

#[whichever::dispatch(Encoder<Json>)]
enum Kind {
    Compact(Compact),
}

fn main() {}
