# Docs

What to read when.

- **What is this, and why is it shaped this way?**
  [vision.md](vision.md) is the argument, the five rules with what holds
  each of them, and what the crate will not be.
- **Why this and not the alternative?**
  [decisions/](decisions/) has one file per choice: the context, the
  decision, the evidence, what was not chosen, and what would reopen it.
- **How is the macro crate built?**
  [../macros/ARCHITECTURE.md](../macros/ARCHITECTURE.md): how an expansion
  flows through the four proc macros, the module map, the forwarding
  engine, the protocol between the carriers, and the invariants every
  change keeps.
- **Coming from `enum_dispatch`?**
  [moving-from-enum-dispatch.md](moving-from-enum-dispatch.md) is the swap,
  attribute for attribute, and the three things to watch.
- **How does it compare with the other crates?**
  [comparison.md](comparison.md) says where `enum_dispatch` and the crates
  after it stop, with the full table probed against each.
- **What does the attribute write?**
  [what-is-generated.md](what-is-generated.md) shows the expansion of one
  example and what each trait item becomes.
- **What can the attribute be told?**
  [options.md](options.md) lists every option on the enum and its variants;
  [routes.md](routes.md) is how an arm reaches a field of the payload or
  what a method returns.
- **What about methods only some variants answer, or the enum answers
  itself?**
  [inherent-methods.md](inherent-methods.md) forwards an inherent method to
  some variants with a fallback;
  [partial-impl-blocks.md](partial-impl-blocks.md) writes some of a trait's
  methods by hand and forwards the rest.
- **Is the trait or the enum another crate's?**
  [restatements.md](restatements.md): traits from `std` and other crates,
  and enums from other crates, each restated once.
- **Is the enum behind a public struct?**
  [structs.md](structs.md) forwards a trait from a struct to one field.
- **What can it not do?**
  [limits.md](limits.md), one line per limit, each with its way around
  where there is one.
- **What does it need, and what does it add to my crate?**
  [compatibility.md](compatibility.md): `no_std`, the minimum Rust version,
  the dependency tree, and the carrier macro a `pub` trait exports.
- **Where is the API reference?**
  On [docs.rs](https://docs.rs/whichever), which carries the same material
  as these pages with runnable examples.
- **When is an enum the right tool, and what does it cost?**
  [when-to-use-enum-dispatch.md](when-to-use-enum-dispatch.md) sets an enum
  against a trait object and a generic, with the measurement behind each
  piece of advice.
- **How fast, measured how?**
  [../benchmarks/README.md](../benchmarks/README.md) says what is compared
  with what and how, so the numbers can be trusted and reproduced;
  [../benchmarks/RESULTS.md](../benchmarks/RESULTS.md) is the last run, with
  its machine, toolchain and date.
- **What changed?**
  [../CHANGELOG.md](../CHANGELOG.md), one entry per release.
- **Who taught this crate what?**
  [../ACKNOWLEDGEMENTS.md](../ACKNOWLEDGEMENTS.md), one lesson per credit.
- **Working on the crate itself.**
  [../CLAUDE.md](../CLAUDE.md) is the rules of the project, and
  [../RELEASING.md](../RELEASING.md) the release checklist.
