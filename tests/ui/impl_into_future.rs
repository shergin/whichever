use std::future::IntoFuture;

#[whichever::dispatchable]
trait Fetch {
    fn fetch(&self) -> impl IntoFuture<Output = u32>;
}

struct Memory;

impl Fetch for Memory {
    fn fetch(&self) -> impl IntoFuture<Output = u32> {
        async { 1 }
    }
}

#[whichever::dispatch(Fetch)]
enum Source {
    Memory(Memory),
}

fn main() {}
