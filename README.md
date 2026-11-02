# whichever

Implements a trait for an enum by forwarding to whichever variant it holds.

```rust
// In the crate that owns the trait.
#[whichever::dispatchable]
pub trait Swap {
    fn swap(&self, amount: u64) -> u64;
    fn freshness(&self) -> u64 {
        0
    }
}

// In any crate that owns an enum over implementors.
#[whichever::dispatch(Swap)]
pub enum SwapKind {
    Uniswap(UniswapSwap),
    Binance(BinanceSwap),
}
```

The enum gets `impl Swap for SwapKind` with one `match` per method, each
arm a fully qualified call into the variant's payload, plus `From<Payload>`
for every variant. The trait and the enum may live in different crates.

Pre-release: the attributes work and are used in production code; the
documentation, the error-message tests and the comparison with other crates
are still to come.
