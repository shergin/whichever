//! Mirrors the Salaris dispatch enums in `ocean` over traits from another crate.
#![allow(async_fn_in_trait)]
use smoke_engine::{
    Amount, AssetReference, NetworkConnector, Registry, RoutePlan, Settler, SmartContract, Swap,
    SwapReference, World,
};
use whichever::{dispatch, dispatchable};

// ---- swaps: `&Self` argument with a mismatch fallback, forwarded defaults ----

#[derive(Debug, Clone)]
pub struct UniswapSwap {
    pub reference: SwapReference,
    pub input: AssetReference,
    pub output: AssetReference,
    pub rate: Amount,
}
impl Swap for UniswapSwap {
    fn swap_reference(&self) -> &SwapReference {
        &self.reference
    }
    fn input_asset(&self) -> &AssetReference {
        &self.input
    }
    fn output_asset(&self) -> &AssetReference {
        &self.output
    }
    fn swap(&self, amount: &Amount) -> Amount {
        amount * self.rate
    }
    fn pricing_eq(&self, other: &Self) -> bool {
        self.rate == other.rate
    }
    fn freshness(&self) -> u64 {
        7
    }
}
#[derive(Debug, Clone)]
pub struct BinanceSwap {
    pub reference: SwapReference,
    pub input: AssetReference,
    pub output: AssetReference,
    pub price: Amount,
}
impl Swap for BinanceSwap {
    fn swap_reference(&self) -> &SwapReference {
        &self.reference
    }
    fn input_asset(&self) -> &AssetReference {
        &self.input
    }
    fn output_asset(&self) -> &AssetReference {
        &self.output
    }
    fn swap(&self, amount: &Amount) -> Amount {
        amount / self.price
    }
}

#[dispatch(Swap)]
#[derive(Debug, Clone)]
pub enum SwapKind {
    UniswapSwap(UniswapSwap),
    BinanceSwap(BinanceSwap),
}

// ---- connectors: differing associated `Error` types unified through `Into` ----

#[derive(Debug, PartialEq)]
pub struct ConnectorError(pub String);
#[derive(Debug)]
pub struct PegboardError;
impl From<PegboardError> for ConnectorError {
    fn from(_: PegboardError) -> Self {
        ConnectorError("pegboard".into())
    }
}
pub struct EthereumConnector;
impl NetworkConnector for EthereumConnector {
    type Error = ConnectorError;
    fn network(&self) -> u8 {
        1
    }
    async fn most_recent_state_reference(&self) -> Result<u64, ConnectorError> {
        Ok(100)
    }
    async fn watch(&self) -> Result<(), ConnectorError> {
        Ok(())
    }
}
pub struct PegboardConnector;
impl NetworkConnector for PegboardConnector {
    type Error = PegboardError;
    fn network(&self) -> u8 {
        9
    }
    async fn most_recent_state_reference(&self) -> Result<u64, PegboardError> {
        Err(PegboardError)
    }
    async fn watch(&self) -> Result<(), PegboardError> {
        Ok(())
    }
}

#[dispatch(NetworkConnector, Error = ConnectorError)]
pub enum NetworkConnectorKind {
    EthereumConnector(EthereumConnector),
    PegboardConnector(PegboardConnector),
}

// ---- settlers: a generic trait instantiated with a concrete world ----

pub struct Solaris;
impl World for Solaris {
    type Plan = u32;
}
pub struct Account(pub u64);
#[derive(Debug)]
pub struct SettlerError;
pub struct Uniswap2Router;
pub struct ZeroExSettler;
impl Settler<Solaris> for Uniswap2Router {
    type Sender = Account;
    type Transaction = u8;
    type Error = SettlerError;
    async fn prepare_transactions(
        &self,
        amount_in: Amount,
        sender: &Account,
    ) -> Result<Vec<u8>, SettlerError> {
        Ok(vec![amount_in as u8, sender.0 as u8])
    }
    fn supports_swap_instruction(&self, instruction: u32) -> bool {
        instruction < 10
    }
}
impl Settler<Solaris> for ZeroExSettler {
    type Sender = Account;
    type Transaction = u8;
    type Error = SettlerError;
    async fn prepare_transactions(
        &self,
        amount_in: Amount,
        sender: &Account,
    ) -> Result<Vec<u8>, SettlerError> {
        Ok(vec![0, amount_in as u8, sender.0 as u8])
    }
    async fn quote_output(&self, route_plan: &RoutePlan<Solaris>) -> Result<Amount, SettlerError> {
        Ok(route_plan.0 as Amount)
    }
    fn supports_swap_instruction(&self, instruction: u32) -> bool {
        instruction < 100
    }
}

#[dispatch(Settler<Solaris>)]
pub enum SettlerKind {
    Uniswap2Router(Uniswap2Router),
    ZeroExSettler(ZeroExSettler),
}

// ---- smart contracts: a foreign trait and a local trait on one enum ----

#[dispatchable]
pub trait SwapSmartContractErased: SmartContract {
    fn swaps(&self) -> Vec<SwapKind>;
}

#[derive(Debug, Clone)]
pub struct Pool2(pub u64);
impl Pool2 {
    /// Same name as the trait method: a method-call forward would pick this one.
    pub fn name(&self) -> &'static str {
        "inherent"
    }
}
impl SmartContract for Pool2 {
    fn account(&self) -> &u64 {
        &self.0
    }
    fn with_account(&self, account: u64) -> Self {
        Pool2(account)
    }
    fn name(&self) -> &'static str {
        "pool2"
    }
}
impl SwapSmartContractErased for Pool2 {
    fn swaps(&self) -> Vec<SwapKind> {
        vec![]
    }
}
#[derive(Debug, Clone)]
pub struct Pool3(pub u64);
impl SmartContract for Pool3 {
    fn account(&self) -> &u64 {
        &self.0
    }
    fn with_account(&self, account: u64) -> Self {
        Pool3(account)
    }
    async fn warmup(&mut self) -> Result<(), String> {
        self.0 += 1;
        Ok(())
    }
}
impl SwapSmartContractErased for Pool3 {
    fn swaps(&self) -> Vec<SwapKind> {
        vec![]
    }
}

#[dispatch(SmartContract)]
#[dispatch(SwapSmartContractErased)]
#[derive(Debug, Clone)]
pub enum SwapSmartContractKind {
    Pool2(Pool2),
    Pool3(Pool3),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniswap(rate: Amount) -> UniswapSwap {
        UniswapSwap {
            reference: SwapReference(1),
            input: AssetReference(1),
            output: AssetReference(2),
            rate,
        }
    }
    fn binance() -> BinanceSwap {
        BinanceSwap {
            reference: SwapReference(2),
            input: AssetReference(2),
            output: AssetReference(1),
            price: 2,
        }
    }

    #[test]
    fn swap_kind_forwards_defaults_and_matches_same_variant() {
        let a: SwapKind = uniswap(3).into();
        let b: SwapKind = binance().into();
        assert_eq!(a.swap(&2), 6);
        assert_eq!(b.swap(&8), 4);
        assert_eq!(a.freshness(), 7);
        assert_eq!(b.freshness(), 0);
        assert_eq!(a.direction(), (AssetReference(1), AssetReference(2)));
        assert!(a.pricing_eq(&uniswap(3).into()));
        assert!(!a.pricing_eq(&uniswap(4).into()));
        assert!(!a.pricing_eq(&b));
        assert_eq!(a.fee(), smoke_engine::Fee(0));
    }

    #[test]
    fn connector_kind_converts_the_error_type() {
        let pegboard: NetworkConnectorKind = PegboardConnector.into();
        assert_eq!(pegboard.network(), 9);
        let error = pollster::block_on(pegboard.most_recent_state_reference()).unwrap_err();
        assert_eq!(error, ConnectorError("pegboard".into()));
        let ethereum: NetworkConnectorKind = EthereumConnector.into();
        assert_eq!(
            pollster::block_on(ethereum.most_recent_state_reference()).unwrap(),
            100
        );
        assert!(pollster::block_on(ethereum.watch()).is_ok());
    }

    #[test]
    fn settler_kind_dispatches_a_generic_trait() {
        let zeroex: SettlerKind = ZeroExSettler.into();
        assert!(zeroex.supports_all(&[1, 50]));
        assert!(!SettlerKind::from(Uniswap2Router).supports_all(&[1, 50]));
        assert_eq!(
            pollster::block_on(zeroex.quote_output(&RoutePlan(42))).unwrap(),
            42
        );
        assert_eq!(
            pollster::block_on(zeroex.prepare_transactions(5, &Account(7))).unwrap(),
            vec![0, 5, 7]
        );
        assert_eq!(zeroex.plan_summary(&3), "3");
    }

    struct AllKnown;
    impl Registry for AllKnown {
        fn known(&self, _: u64) -> bool {
            true
        }
    }

    #[test]
    fn smart_contract_kind_implements_two_traits() {
        let pool2: SwapSmartContractKind = Pool2(1).into();
        assert_eq!(SmartContract::name(&pool2), "pool2");
        assert_eq!(pool2.name(), "pool2");
        assert!(matches!(
            pool2.with_account(5),
            SwapSmartContractKind::Pool2(Pool2(5))
        ));
        assert_eq!(pool2.apply(|account| account + 1), 2);
        assert!(pollster::block_on(pool2.validate(&AllKnown)).is_ok());
        let mut pool3: SwapSmartContractKind = Pool3(1).into();
        pollster::block_on(pool3.warmup()).unwrap();
        assert_eq!(*pool3.account(), 2);
        assert_eq!(pool3.name(), "default");
        assert!(pool3.swaps().is_empty());
    }
}
