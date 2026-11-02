#[whichever::dispatchable]
trait Items {
    fn items(&self) -> impl Iterator<Item = u32> + '_;
}

struct Few(Vec<u32>);
struct One(Option<u32>);

impl Items for Few {
    fn items(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied()
    }
}

impl Items for One {
    fn items(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied()
    }
}

#[whichever::dispatch(Items)]
enum Source {
    Few(Few),
    One(One),
}

fn main() {}
