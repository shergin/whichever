//! `#[dispatchable]` above `#[async_trait]` would capture `async fn`
//! signatures that the trait no longer has; it goes below.
use async_trait::async_trait;

#[whichever::dispatchable]
#[async_trait]
trait Fetch {
    async fn fetch(&self) -> u32;
}

fn main() {}
