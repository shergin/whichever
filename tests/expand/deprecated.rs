//! A deprecated method or variant is forwarded under `#[allow(deprecated)]`.
#[whichever::dispatchable]
pub trait Versioned {
    #[deprecated(note = "use `name`")]
    fn kind(&self) -> &'static str;
    fn name(&self) -> &'static str;
}

pub struct Old;
pub struct New;

impl Versioned for Old {
    fn kind(&self) -> &'static str {
        "old"
    }
    fn name(&self) -> &'static str {
        "old"
    }
}

impl Versioned for New {
    fn kind(&self) -> &'static str {
        "new"
    }
    fn name(&self) -> &'static str {
        "new"
    }
}

#[whichever::dispatch(Versioned)]
pub enum Any {
    #[deprecated(note = "use `New`")]
    Old(Old),
    New(New),
}

fn main() {}
