# `From` is opt-out and `TryFrom` is opt-in

Status: accepted, 2026-11-01 (`From` in 0.1.0, `try_from` in 0.2.0, per
variant in 0.4.0; the by-reference forms `try_from = ref` and `ref_mut`,
and `into = Target`, on 2026-11-05, after sound_flow and mindus met the
by-reference condition and screeps and boreholeio the `into` one). Serves
rule 1 of the [vision](../vision.md), the output is the hand's. Reopen if
the surveys show `TryFrom` wanted more often than not.

## Context

`enum_dispatch` generates `From<Payload>` and `TryInto<Payload>` for every
variant unconditionally, the latter with a `&'static str` error. The surveys
found the `TryInto` impls public and unused in project after project: 75 in
edtui, 22 in hugr-core, 13 in rspotify-model, 2 in amdgpu-sysfs. They also
found payloads that should not convert into the enum (a `String` variant on
an error enum), and payload types held by two variants.

## Decision

`From<Payload> for Enum` is generated per payload type, on by default, and
left out with `from = false` on the enum or on one variant. `TryFrom<Enum>
for Payload` is off by default and added with `try_from = true` on the enum
or on one variant, with the enum itself as the error, so a mismatch hands
the value back. A payload type held by two variants gets neither. A struct
gets no conversions.

The by-reference forms borrow for a lifetime of their own and hand the
reference back as the error, `TryFrom<&'a Enum> for &'a Payload` with
`Error = &'a Enum`, which is what hugr-core and sound_flow write.
`into = Target` writes `From<Enum> for Target` with `Into::into(payload)`
in every arm, the conversion analogue of forwarding; a variant without a
payload, or left out of the conversions, is one sentence.

## Evidence

- The survey log's "drops N unused public `TryInto`" verdicts.
- The planning note `notes/gaps/conversions.md`, section 4.
- `tests/expand/basic.rs` for `From`; `tests/ui/struct_from_option.rs` and
  `tests/ui/from_overlap_type_parameter.rs` pin the limits.

## Not chosen

- `TryInto` with a `&'static str` error: loses the value, and the message
  is not the user's.
- Conversions always on: the unused public impls above, and a `From` for a
  payload that should not have one.
- By-reference `TryFrom` left out: hugr-core, sound_flow and mindus
  write it by hand, and `enum_delegate` generates it for every user.
- `TryIntoError<T>` as the error, as `derive_more` has: the enum itself is
  what a hand returns, and is `Debug` when the enum is.
