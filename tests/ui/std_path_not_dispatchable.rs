struct Celsius(f64);

impl std::fmt::Display for Celsius {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} C", self.0)
    }
}

#[whichever::dispatch(std::fmt::Display)]
enum Temperature {
    Celsius(Celsius),
}

fn main() {}
