# A boxed future is rewrapped by boxing

Status: accepted, 2026-11-06. Serves rule 1 of the [vision](../vision.md),
the output is the hand's, and reads its "a conversion the signature does
not ask for": the signature of an `#[async_trait]` method asks for a
`Pin<Box<dyn Future>>`, so the box is the hand's. Reopen if the attribute's
desugaring changes its spelling, which the recogniser follows by name.

## Context

`#[async_trait]` rewrites `async fn load(&self) -> Result<u32, Self::Error>`
into a method returning `Pin<Box<dyn Future<Output = Result<u32,
Self::Error>> + Send + 'async_trait>>`. With `Error` fixed on the enum the
arms' outputs need `map_err(Into::into)`, which lives inside the future;
until 2026-11-06 the boxed return was forwarded as a plain type, and rustc
reported the mismatch at the trait's attribute (`notes/gaps/self-returns.md`
section 4; activitypub_federation's `ActivityHandler` with a unified error,
and Lemmy over it). The README's claim that differing associated types
unify through `Into` did not hold for such traits, where it is wanted most.

## Decision

A return of the shape `Pin<Box<dyn Future<Output = T> + ..>>` is a future
whose output is `T`, as `impl Future<Output = T>` is. Where `T` needs no
conversion, the arms return the payloads' futures as they are: one type for
every arm, nothing to wrap. Where it does, the arms are awaited in one
`async move` block, converted there, and the block is boxed and pinned
again with the `Box` the signature names, `Box::pin(async move { match ..
})`, which is what `#[async_trait]` itself writes for an `async fn` of the
impl. The allocation is the one the signature asks for. The same reading
admits `Box::new` for a `-> Self` through a `Box` variant, whose shape asks
for the box; that stays watched, with loro as its one case.

## Evidence

- `tests/expand/boxed_futures.rs`: the plain forward and the converted one,
  spelled with the signature's own `Box`.
- `tests/behaviour/async_trait_above_dispatchable.rs`, `the_boxed_futures_output_is_converted`,
  under the attribute itself.
- The unit test `a_boxed_future_is_awaited_only_to_convert_its_output` in
  `macros/src/shape.rs`.

## Not chosen

- A new box around every boxed future: a second allocation where nothing
  converts, which the signature does not need.
- A sentence naming a partial block: an `async fn` written in the block
  under `#[async_trait]` works, and is the whole `match` by hand.
- `::std::boxed::Box::pin`: the crate is `no_std` and names only `core`
  items; the signature's own `Box` is the one it asks for, under whatever
  path the trait spelled.
