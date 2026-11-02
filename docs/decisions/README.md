# Decisions

Choices made among real alternatives, with the evidence that decided them
and the condition that would reopen them. One file per decision.

These are not rules. The rules in the [vision](../vision.md) must not rot;
a decision may be superseded, and says so in its status line. A decision
never overrides the vision; where a rule rests on a decision, the vision
names it.

Each record has the same shape: a status line (accepted or superseded, with
the date, the rule it serves, and what would reopen it), then Context,
Decision, Evidence, Not chosen. Keep them short; the argument belongs in
the vision, the proof belongs here. Where a record grew out of a gap note in
the planning notes, the note holds the long form and the record names it.

- [The trait travels in a macro, not a cache](the-trait-travels-in-a-macro.md)
- [The carrier macro's name and home](the-carrier-macros-name-and-home.md)
- [Receivers that would allocate are rejected](receivers-that-allocate-are-rejected.md)
- [`impl Trait` returns are rejected, except futures](impl-trait-returns-are-rejected-except-futures.md)
- [Associated consts are checked for four types](associated-consts-are-checked-for-four-types.md)
- [A rejected enum is still emitted](a-rejected-enum-is-still-emitted.md)
- [Foreign traits are restated, and the common ones ship](foreign-traits-are-restated.md)
- [`From` is opt-out and `TryFrom` is opt-in](from-is-opt-out-and-try-from-is-opt-in.md)
- [A variant without a payload is left to the fallback](a-variant-without-a-payload-is-left-to-the-fallback.md)
- [A struct forwards through a marked field](a-struct-forwards-through-a-marked-field.md)
- [Benchmarks are runtime only](benchmarks-are-runtime-only.md)
