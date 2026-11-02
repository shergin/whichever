type Fallible<T> = Result<T, u8>;

#[whichever::dispatchable]
trait Rebuild {
    fn rebuild(&self) -> Fallible<Self>
    where
        Self: Sized;
}

struct Small;

impl Rebuild for Small {
    fn rebuild(&self) -> Fallible<Self> {
        Ok(Small)
    }
}

#[whichever::dispatch(Rebuild)]
enum Number {
    Small(Small),
}

fn main() {}
