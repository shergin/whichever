# Compared with the alternatives

Several crates write the forwarding impl of a trait for an enum over its
implementors, the `match` a hand would otherwise write for every method.

`enum_dispatch` came first, and its mechanism is its limit. It keeps the
trait's definition in a process-global cache inside the proc-macro server,
and remembers it from one expansion to the next. That is why:

- **An enum in another crate gets no impl, silently.** The cache is empty
  when the enum's crate compiles, so the enum compiles without the trait and
  the first call fails, far from the cause.
- **Real trait shapes panic or are refused.** Associated types, associated
  consts, methods without a receiver, `&Self` arguments and lifetime-generic
  traits are out of reach.
- **Errors lose their spans.** Tokens replayed from a cache no longer point
  at your code.

The crates that followed work across crates, but each supports a different
slice of what real traits look like:

- `ambassador` handles associated types and generic traits, but no `Self`
  in arguments or returns, no associated consts and no `From` impls.
- `enum_delegate` handles `-> Self` and unifies associated types, but
  panics at runtime on `&Self` arguments, has no `async`, generic traits or
  generic methods, and leaves out `#[inline]`, which costs an out-of-line
  call across a crate boundary ([measured](../benchmarks/RESULTS.md)).
- `delegate` handles almost anything, because you list every signature
  again by hand.
- `portrait`, `delegation` and `static-dispatch` cover plain `&self`
  methods, defaults and generic methods.

Whichever you pick, the first trait with a `&Self` argument or an
associated const sends you back to writing the `match` by hand.

`whichever` keeps nothing between expansions. `#[dispatchable]` exports a
carrier macro holding the trait's signatures, re-exported under the trait's own
name, and `#[dispatch]` calls it. The trait reaches the enum through
ordinary name resolution, so it works across crates, in any order, and
every span is the one you wrote.

Every row where only `whichever` says yes is a trait that, with any other
crate, goes back to a hand-written `match`.

| | whichever | enum_dispatch 0.3 | ambassador 0.5 | portrait 0.3 | enum_delegate 0.2 | delegation 0.5 | static-dispatch 0.3 | delegate 0.13 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Trait and enum in different crates | yes | no, silently | yes | yes | yes | yes | yes | yes, signatures re-listed |
| Traits of `std` or a crate you do not own | yes, `Display`, `io::Read` and more shipped; others restated once | no | yes, restated | - | - | - | - | yes, signatures re-listed |
| Default methods forwarded | yes | yes | yes | yes | yes | yes | yes | if listed |
| `Self`, `&Self`, `&mut Self` arguments | yes, with a declared fallback | no | no | no | panics at runtime | no | no | no |
| `-> Self` | yes | yes | no | no | yes | no | no | with `#[into]` |
| `-> Option<Self>`, `-> Result<Self, E>` | yes | no | - | - | - | - | - | - |
| Associated types | yes | no | yes | no | yes | no | no | by hand |
| Differing associated types unified | yes, through `Into` | no | no | no | yes, wrapper enum | no | no | by hand |
| Associated consts | yes, checked to agree or fixed on the enum | no | no | no | no | no | no | by hand |
| Methods without a receiver | with a default body, or written by hand next to the forwarded ones | no | no | no | no | no | no | - |
| Some methods written by hand, the rest forwarded | yes | no | no | no | no | no | no | yes, by listing the rest |
| `async fn` | yes | yes | yes | no | no | no | yes | yes |
| Generic trait at a concrete instantiation | yes | no | yes | no | no | yes | no | by hand |
| Const generics on the trait | yes | - | - | - | - | - | - | - |
| Generic methods | yes | yes | yes | yes | no | yes | yes | yes |
| Lifetime-generic trait and enum | yes | no | yes | no | no | yes | no | by hand |
| Several traits on one enum | yes | yes | yes | yes | no | no | yes | yes |
| Payload with a same-named inherent method | safe | safe | safe | safe | safe | safe | safe | calls it, unless `#[through]` |
| `Box<T>`, `Rc<T>`, `&T` payloads | yes, `#[dispatch(deref)]` | no | no | no | no | no | no | yes |
| A field of the payload, or of a variant with several fields, as the target | yes, a route or a marked field | no | no | no | no | no | no | yes, by hand |
| `-> impl Trait` | `impl Future<Output = T>` on an enum; any, through a struct | no | through a struct | no | no | no | no | no |
| `From<Payload>` impls | yes, opt-out; `TryFrom<Enum>` opt-in, by value, `&` and `&mut`; `From<Enum> for Target` with `into =` | yes, plus a forced `TryInto` | no | no | yes, with `&` and `&mut` `TryInto` | no | no | no |
| Inherent methods forwarded to some variants, with a fallback | yes | no | no | no | no | no | no | by hand |
| Variants without a payload, left to the fallback | yes | no | - | - | - | - | - | - |
| A struct forwarding to one field | yes | no | yes | yes | no | yes | no | yes |
| An enum another crate defines | yes, restated with `remote` | no | yes, restated | no | yes, restated | with `unsafe` | no | no |
| `self: Pin<&mut Self>`, `self: Box<Self>` | yes | - | - | - | - | - | - | - |

Probed on 2026-11-01 against the versions shown, with the trait in one
crate and the enum in another, on stable Rust. A dash means the case was
not probed for that crate. The `whichever` column is what `tests/` checks.
