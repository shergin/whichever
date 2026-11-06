//! `#[async_trait]` goes above `#[dispatchable]`: the carrier then holds the
//! signatures as `async_trait` rewrote them, which is what every impl has.

use async_trait::async_trait;

#[async_trait]
#[whichever::dispatchable]
pub trait Fetch {
    async fn fetch(&self) -> u32;
    fn sync(&self) -> u32 {
        0
    }
}

pub struct Fast;
pub struct Slow;

#[async_trait]
impl Fetch for Fast {
    async fn fetch(&self) -> u32 {
        1
    }
}

#[async_trait]
impl Fetch for Slow {
    async fn fetch(&self) -> u32 {
        2
    }
    fn sync(&self) -> u32 {
        2
    }
}

#[whichever::dispatch(Fetch)]
pub enum Any {
    Fast(Fast),
    Slow(Slow),
}

#[test]
fn the_boxed_future_is_forwarded_as_it_is() {
    let slow: Any = Slow.into();
    assert_eq!(pollster::block_on(slow.fetch()), 2);
    assert_eq!(slow.sync(), 2);
    assert_eq!(pollster::block_on(Any::from(Fast).fetch()), 1);
}
