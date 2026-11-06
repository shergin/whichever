# A struct's field may name its traits

Status: accepted, 2026-11-05. Supersedes
[A struct forwards through a marked field](a-struct-forwards-through-a-marked-field.md),
whose reopening condition rsynko, sabicom, activityrust and russh met.
Serves rule 3 of the [vision](../vision.md), what earns a shape. Reopen if a
project needs one trait forwarded to two fields at once, which no match can
express.

## Context

The marked field was the target of every trait the struct dispatched.
rsynko's `LawEnv` sends 63 traits to 7 fields, sabicom's `Inner4` sends
`Rom` to `rom` and `Interrupt` to `signals`, activityrust's five actor
structs and russh's `ChannelStream` the same, all with ambassador's
`target =` and all by hand under whichever. The superseded record's "Not
chosen" named the spelling, the trait on the field, and the reason it was
not chosen then: no surveyed project did it.

## Decision

A field's `#[dispatch(Trait, ..)]` names the traits it is the target of;
the field marked bare, `#[dispatch]`, takes every trait not named
elsewhere, and a struct with one field needs no mark. The struct's
attribute still names every trait it dispatches, so the impl's home stays
the item that gets it, which the superseded record's reason was and still
is. A trait named on two fields is one sentence; so is a trait named on a
field and not on the struct, since the mark would otherwise be dead.

## Evidence

- The sweep of the ambassador surveys in the review of 2026-11-05 (item 1
  of its list of shapes) and section 4.4.
- `tests/expand/per_trait_fields.rs`; `tests/behaviour.rs`,
  `per_trait_fields`; `tests/ui/trait_named_on_two_fields.rs`,
  `field_names_an_undispatched_trait.rs`.

## Not chosen

- `#[dispatch(Trait, target = field)]` on the struct: the trait's story
  in one place, but the field's type is where a reader looks for what it
  does, and the marker was already there.
- Dropping the trait from the struct's attribute when a field names it:
  the impl's home would then be a field.
