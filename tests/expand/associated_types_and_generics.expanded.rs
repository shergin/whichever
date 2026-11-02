pub trait World {
    type Plan;
}
pub struct Solaris;
impl World for Solaris {
    type Plan = u32;
}
pub trait Settler<W: World> {
    type Error;
    async fn settle(&self, plan: &W::Plan) -> Result<u64, Self::Error>;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Settler_1e381856dbf52faf_11_1 as Settler;
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
pub enum SettlerKind {
    Router(Router),
    Aggregator(Aggregator),
}
#[automatically_derived]
impl ::core::convert::From<Router> for SettlerKind {
    #[inline]
    fn from(value: Router) -> Self {
        Self::Router(value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Aggregator> for SettlerKind {
    #[inline]
    fn from(value: Aggregator) -> Self {
        Self::Aggregator(value)
    }
}
#[automatically_derived]
impl Settler<Solaris> for SettlerKind {
    type Error = u16;
    #[inline]
    async fn settle(
        &self,
        __argument0: &<Solaris as World>::Plan,
    ) -> Result<u64, Self::Error> {
        match self {
            SettlerKind::Router(__inner) => {
                <Router as Settler<Solaris>>::settle(__inner, __argument0)
                    .await
                    .map_err(::core::convert::Into::into)
            }
            SettlerKind::Aggregator(__inner) => {
                <Aggregator as Settler<Solaris>>::settle(__inner, __argument0)
                    .await
                    .map_err(::core::convert::Into::into)
            }
        }
    }
}
fn main() {}
