# whichever

[![crates.io](https://img.shields.io/crates/v/whichever.svg)](https://crates.io/crates/whichever)
[![docs.rs](https://docs.rs/whichever/badge.svg)](https://docs.rs/whichever)
[![CI](https://github.com/shergin/whichever/actions/workflows/ci.yml/badge.svg)](https://github.com/shergin/whichever/actions/workflows/ci.yml)

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

`SwapKind` now implements `Swap` with one `match` per method, each arm a
fully qualified call into the variant's payload, and gets `From<UniswapSwap>`
and `From<BinanceSwap>`:

```rust
impl Swap for SwapKind {
    #[inline]
    fn swap(&self, amount: u64) -> u64 {
        match self {
            SwapKind::Uniswap(inner) => <UniswapSwap as Swap>::swap(inner, amount),
            SwapKind::Binance(inner) => <BinanceSwap as Swap>::swap(inner, amount),
        }
    }
    #[inline]
    fn freshness(&self) -> u64 {
        match self {
            SwapKind::Uniswap(inner) => <UniswapSwap as Swap>::freshness(inner),
            SwapKind::Binance(inner) => <BinanceSwap as Swap>::freshness(inner),
        }
    }
}
```

That is the code a careful hand would write: no trait object, no closure,
no conversion, and a variant's override of a default method is never hidden
by the default.

## Why another one

An enum over the implementors of a trait is the cheapest way to hold "one
of these": no allocation, no vtable, and the compiler sees every arm. The
forwarding impl is the price, paid again for every method and every variant.

`enum_dispatch` made it automatic, but it keeps the trait's definition in a
process-global cache inside the proc-macro server. That is why it cannot see
a trait from another crate (the enum compiles with no impl and the first
call fails), panics on associated types and receiver-less methods, and
loses the spans of its errors. `whichever` keeps nothing between
expansions: `#[dispatchable]` exports a hidden macro carrying the trait's
tokens, re-exported under the trait's own name, and `#[dispatch]` calls it.
The trait and the enum may live in different crates, in any order, and the
shapes real traits have (associated types, `&Self` arguments, generic
traits, several traits on one enum) dispatch too.

| | whichever | enum_dispatch 0.3 | ambassador 0.5 | portrait 0.3 | enum_delegate 0.2 | delegation 0.5 | static-dispatch 0.3 | delegate 0.13 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Trait and enum in different crates | yes | no, silently | yes | yes | yes | yes | yes | yes, signatures re-listed |
| Default methods forwarded | yes | yes | yes | yes | yes | yes | yes | if listed |
| `Self`, `&Self`, `&mut Self` arguments | yes, with a declared fallback | no | no | no | panics at runtime | no | no | no |
| `-> Self` | yes | yes | no | no | yes | no | no | with `#[into]` |
| `-> Option<Self>`, `-> Result<Self, E>` | yes | no | - | - | - | - | - | - |
| Associated types | yes | no | yes | no | yes | no | no | by hand |
| Differing associated types unified | yes, through `Into` | no | no | no | yes, wrapper enum | no | no | by hand |
| Associated consts | yes | no | no | no | no | no | no | by hand |
| Methods without a receiver | with a default body | no | no | no | no | no | no | - |
| `async fn` | yes | yes | yes | no | no | no | yes | yes |
| Generic trait at a concrete instantiation | yes | no | yes | no | no | yes | no | by hand |
| Const generics on the trait | yes | - | - | - | - | - | - | - |
| Generic methods | yes | yes | yes | yes | no | yes | yes | yes |
| Lifetime-generic trait and enum | yes | no | yes | no | no | yes | no | by hand |
| Several traits on one enum | yes | yes | yes | yes | no | no | yes | yes |
| Payload with a same-named inherent method | safe | safe | safe | safe | safe | safe | safe | calls it, unless `#[through]` |
| `Box<T>` payloads | planned | no | no | no | no | no | no | yes |
| `-> impl Trait` | no | no | no | no | no | no | no | no |
| `From<Payload>` impls | yes, opt-out | yes, plus a forced `TryInto` | no | no | yes | no | no | no |

Probed on 2026-11-01 against the versions shown, with the trait in one
crate and the enum in another, on stable Rust. A dash means the case was
not probed for that crate. The `whichever` column is what `tests/` checks.

## Options

All options follow the trait path in `#[dispatch(..)]`.

- `Name = Type` fixes the associated type `Name` instead of taking it from
  the first variant. A method returning `Self::Name` is converted with
  `Into`, one returning `Result<_, Self::Name>` with `map_err(Into::into)`,
  so variants with their own error types can share one enum:

  ```rust
  #[whichever::dispatch(Connector, Error = NetworkError)]
  pub enum ConnectorKind {
      Mainnet(MainnetConnector),  // Error = NetworkError
      Local(LocalConnector),      // Error = io::Error, which is Into<NetworkError>
  }
  ```

- `mismatch(method = expr, ..)` gives the result of `method` when its
  `Self` arguments hold a different variant than `self`. The trait author
  can decide once instead, with `#[dispatch(mismatch = expr)]` on the
  method:

  ```rust
  #[whichever::dispatchable]
  pub trait Swap {
      /// Two different venues never price alike.
      #[dispatch(mismatch = false)]
      fn pricing_eq(&self, other: &Self) -> bool;
  }
  ```

- `from = false` suppresses the `From<Payload>` impls.

Several traits are dispatched by repeating the attribute; a supertrait is
one more attribute:

```rust
#[whichever::dispatch(SmartContract)]
#[whichever::dispatch(SwapSmartContract)]
pub enum ContractKind { /* .. */ }
```

A generic trait is dispatched at a concrete instantiation:
`#[whichever::dispatch(Settler<Solaris>)]`. Type, lifetime and const
parameters are substituted in every signature, and `W::Plan` becomes
`<Solaris as World>::Plan` when `W` has one bound.

## What each trait item becomes

| Trait item | Enum impl |
| --- | --- |
| `type X;` | `type X = <FirstVariant as Trait>::X;`, or the `X = Type` option. Every other variant is checked to agree; a disagreement is reported on that variant. |
| `const N: T;` | `const N: T = <FirstVariant as Trait>::N;` |
| method with a receiver | A `match` over the variants calling `<Payload as Trait>::method(payload, args..)`, with `#[inline]`. |
| default body | Ignored: forwarded like any other method. |
| `async fn`, `unsafe fn` | Arms are awaited; calls are wrapped in `unsafe { }`. |
| argument `Self`, `&Self`, `&mut Self` | Matched on the same variant as `self`; different variants give the `mismatch` expression. |
| return `Self` | Wrapped back into the same variant; `Option<Self>` and `Result<Self, E>` are mapped. |
| method without a receiver | Left to the trait's default body; an error when there is none. |
| `#[cfg]` on a method or variant | Carried onto the generated method or arm. |

## Limits

- Every variant holds exactly one unnamed field, `Name(Payload)`, and
  every payload implements the trait.
- Return-position `impl Trait` cannot be dispatched by a `match`.
- `Box<T>`, `Arc<T>` and `&T` payloads are not dereferenced for the call
  yet.
- Generic associated types and `self: Type` receivers are not supported.
- The types in the trait's signatures are written into the enum's module
  and must be in scope there, as for a hand-written impl; `crate::` paths
  are rewritten to the trait's crate, `super::` and `self::` paths are not.
- Associated consts are taken from the first variant without a check.

Every unsupported shape is reported as one sentence on the offending trait
item or variant; see `tests/ui` for all of them.

"cannot find macro `Swap` in this scope" at the enum means the trait is not
marked `#[dispatchable]`, or the trait, which carries the macro, is not
imported where the enum is.

## Compatibility

`no_std`, with no dependencies beyond the proc-macro crate. The minimum
supported Rust version is 1.88. Both the crate that owns the trait and the
crate that owns the enum depend on `whichever`; renaming the dependency is
fine.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
