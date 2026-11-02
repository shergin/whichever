pub trait World {
    type Plan;
}

pub struct Solaris;

impl World for Solaris {
    type Plan = u32;
}

#[whichever::dispatchable]
pub trait Settler<W: World> {
    type Error;
    async fn settle(&self, plan: &W::Plan) -> Result<u64, Self::Error>;
}

pub struct Router;
pub struct Aggregator;

impl Settler<Solaris> for Router {
    type Error = u8;
    async fn settle(&self, plan: &u32) -> Result<u64, u8> {
        Ok(u64::from(*plan))
    }
}

impl Settler<Solaris> for Aggregator {
    type Error = u16;
    async fn settle(&self, plan: &u32) -> Result<u64, u16> {
        Ok(u64::from(*plan) * 2)
    }
}

#[whichever::dispatch(Settler<Solaris>, Error = u16)]
pub enum SettlerKind {
    Router(Router),
    Aggregator(Aggregator),
}

fn main() {}
