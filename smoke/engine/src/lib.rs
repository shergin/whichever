//! Mirrors the shapes of the Salaris traits that live below the dispatch
//! enums: `engine::swaps::Swap`, `engine::network_connector::NetworkConnector`,
//! `engine::execution::Settler<W>` and `ethereum::smart_contract::SmartContract`.
#![allow(async_fn_in_trait)]
use std::fmt::Debug;

use whichever::dispatchable;

pub type Amount = u128;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetReference(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapReference(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fee(pub u8);

#[dispatchable]
pub trait Swap: Send + Sync + Debug {
    fn swap_reference(&self) -> &SwapReference;
    fn input_asset(&self) -> &AssetReference;
    fn output_asset(&self) -> &AssetReference;
    fn direction(&self) -> (AssetReference, AssetReference) {
        (*self.input_asset(), *self.output_asset())
    }
    fn swap(&self, amount: &Amount) -> Amount;
    /// Two different variants never price alike, so the enum answers `false`.
    #[dispatch(mismatch = false)]
    fn pricing_eq(&self, other: &Self) -> bool
    where
        Self: Sized,
    {
        let _ = other;
        false
    }
    fn freshness(&self) -> u64 {
        0
    }
    /// Named through `crate::` on purpose: the path must survive the move
    /// into the enum's crate.
    fn fee(&self) -> crate::Fee {
        crate::Fee(0)
    }
}

#[dispatchable]
pub trait NetworkConnector: Send + Sync {
    type Error;
    fn network(&self) -> u8;
    async fn most_recent_state_reference(&self) -> Result<u64, Self::Error>;
    async fn watch(&self) -> Result<(), Self::Error>;
}

pub trait World {
    type Plan: Debug;
}
#[derive(Debug)]
pub struct RoutePlan<W: World>(pub W::Plan);

#[dispatchable]
pub trait Settler<W: World> {
    type Sender: ?Sized;
    type Transaction;
    type Error;
    async fn prepare_transactions(
        &self,
        amount_in: Amount,
        sender: &Self::Sender,
    ) -> Result<Vec<Self::Transaction>, Self::Error>;
    async fn quote_output(&self, route_plan: &RoutePlan<W>) -> Result<Amount, Self::Error> {
        let _ = route_plan;
        Ok(0)
    }
    fn plan_summary(&self, plan: &W::Plan) -> String {
        format!("{plan:?}")
    }
    fn supports_swap_instruction(&self, instruction: u32) -> bool;
    fn supports_all(&self, instructions: &[u32]) -> bool {
        instructions
            .iter()
            .all(|instruction| self.supports_swap_instruction(*instruction))
    }
}

pub trait Registry {
    fn known(&self, account: u64) -> bool;
}

#[dispatchable]
pub trait SmartContract: Send + Sync + Sized + Debug {
    fn account(&self) -> &u64;
    fn with_account(&self, account: u64) -> Self;
    fn name(&self) -> &'static str {
        "default"
    }
    async fn warmup(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn validate(&self, registry: &dyn Registry) -> Result<(), String> {
        if registry.known(*self.account()) {
            Ok(())
        } else {
            Err("unknown".into())
        }
    }
    fn apply<F: Fn(u64) -> u64>(&self, f: F) -> u64 {
        f(*self.account())
    }
    fn deployments() -> &'static [u64] {
        &[]
    }
}
