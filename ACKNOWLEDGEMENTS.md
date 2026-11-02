# Acknowledgements

One specific lesson per credit.

## The crates that came before

- [`enum_dispatch`](https://crates.io/crates/enum_dispatch) showed that a
  macro can write the forwarding impl, and gave the two-attribute shape
  that `whichever` keeps. Its process-global cache, and the three failures
  that follow from it, decided that the trait's tokens would travel in a
  macro instead.
- [`ambassador`](https://crates.io/crates/ambassador) restates a trait
  another crate owns, the precedent for `remote =`, and exports a macro per
  trait at the crate root, which showed the name collision that
  `whichever` hashes around.
- [`portrait`](https://crates.io/crates/portrait) fills an impl from a
  macro the trait emits, the same carrier idea applied to a different
  problem.
- [`delegate`](https://crates.io/crates/delegate) forwards exactly the
  methods you list, which is the measure of how much a user will write to
  keep an impl honest, and what `whichever` had to make unnecessary.
- [`enum_delegate`](https://crates.io/crates/enum_delegate) rewraps `-> Self`
  and unifies differing associated types in a wrapper enum; its random
  macro names and its omitted `#[inline]` are two things the benchmarks
  measured and this crate does differently.
- [`delegation`](https://crates.io/crates/delegation) forwards through a
  chain of `Either`s, which is where the benchmarks learned what walking a
  chain costs at 32 variants.
- [`serde`](https://crates.io/crates/serde)'s `#[serde(remote = "..")]` is
  the word and the shape for restating a type of another crate.

## The tools

- [`trybuild`](https://crates.io/crates/trybuild) and
  [`macrotest`](https://crates.io/crates/macrotest), by David Tolnay, are
  the two contracts: one compile-fail case per limit, one expansion golden
  per shape.
- [`criterion`](https://crates.io/crates/criterion) runs the suite and
  reports the intervals the results file prints.
- [`syn`](https://crates.io/crates/syn), [`quote`](https://crates.io/crates/quote),
  [`proc-macro2`](https://crates.io/crates/proc-macro2) and
  [`proc-macro-crate`](https://crates.io/crates/proc-macro-crate) are the
  macro crate's whole dependency tree.

## The projects surveyed

Each was read, never built, to find where it still forwards by hand. Where a
shape `whichever` could not reach was found, the project is the motivating
case in the planning notes, and the shape that shipped is its lesson.

- [memcache](https://crates.io/crates/memcache): a `Stream` enum whose
  hand-written `Read` and `Write` hid the payloads' overrides, the case for
  restatements and `whichever::foreign`.
- [loro](https://github.com/loro-dev/loro) (`loro-internal`): a trait method
  the enum answers itself, the case for the partial impl block; aliased
  `Result<Self>`; `-> Self` through a boxed payload, still open.
- [fmmap](https://github.com/al8n/fmmap): `-> impl Future<Output = T> +
  Send` forwarded 87 times per runtime, the case for the one-block
  expansion; identical traits emitted twice from one macro, the case for
  the carrier macro's name.
- [Arti](https://gitlab.torproject.org/tpo/core/arti) (`tor-proto`),
  [rattler](https://github.com/conda/rattler) (`rattler_shell`) and
  [proof-of-sql](https://github.com/spaceandtimefdn/sxt-proof-of-sql): a
  public struct wrapping a private dispatch enum and forwarding again by
  hand, the case for struct forwarding; proof-of-sql also for the enum that
  stays emitted on error, for `#[cfg]`-aware shape errors and for variants
  without a payload.
- [celestia-types](https://crates.io/crates/celestia-types): an associated
  const that differs per payload by design, the case for the agreement
  check and `Name = value`.
- [rspotify](https://github.com/ramsayleung/rspotify) (`rspotify-model`):
  inherent methods returning the enum at another lifetime, the case for
  `with = Into::into`; a prelude's glob re-export carrying the carrier
  macro downstream.
- [hugr](https://github.com/CQCL/hugr) (`hugr-core`): `From` overlaps when a
  payload is a bare type parameter; by-reference `TryFrom`, watched.
- [sqruff](https://github.com/quarylabs/sqruff): error spans landing on the
  enum instead of the method; same-named field forwarding, watched.
- [twitch-irc](https://crates.io/crates/twitch-irc): a generic trait at the
  enum's own parameters, which worked and was undersold; collapsing into a
  common target, watched.
- [s390-tools](https://github.com/ibm-s390-linux/s390-tools) (`s390_pv`,
  `pvimg`): an associated type in argument position, and errors landing on
  the first of several `#[dispatch]` attributes.
- [cardinality-estimator](https://crates.io/crates/cardinality-estimator):
  a `pub(crate)` trait's macro reaching the crate root, the case for the
  carrier macro's home.
- [tros](https://crates.io/crates/tros): a trait with a generic associated
  type, the case on file for GATs.
- [light-curve-feature](https://crates.io/crates/light-curve-feature):
  `try_from = true` with `Self` in a payload type, and the broadest shape
  test so far.
- [edtui](https://crates.io/crates/edtui), [zksync_vm2](https://github.com/matter-labs/vm2),
  [regexml](https://crates.io/crates/regexml), [nlprule](https://crates.io/crates/nlprule),
  [transform-gizmo](https://crates.io/crates/transform-gizmo),
  [amdgpu-sysfs](https://crates.io/crates/amdgpu-sysfs),
  [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) and
  [Temporal's Rust SDK](https://github.com/temporalio/sdk-core): the
  surveys whose verdict was "not a candidate", which is what keeps the
  survey log honest, and whose MSRV costs are the reason the floor is
  stated as a contract.

## The shape of these documents

The vision with a mechanism per rule, the decision records with a reopening
condition and the private planning notes follow the author's three earlier
projects, topos, malevich and baton, each of which refined the practice for
the next.
