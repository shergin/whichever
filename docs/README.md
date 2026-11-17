# Docs

What to read when, sorted by the question you have.

## Start here

New to `whichever`? Read these in order:

1. [tutorial.md](tutorial.md), a guided tour, start to finish, in about
   fifteen minutes.
2. [glossary.md](glossary.md), the words these pages use, such as
   payload, route and carrier macro, each in a few words.
3. Then the page for what you need, from the lists below.

## Using the attributes

- **What does the attribute write?**
  [what-is-generated.md](what-is-generated.md) shows the expansion of one
  example and what each trait item becomes.
- **What options can I put on the attribute?**
  [options.md](options.md) lists every option on the enum and its
  variants. [routes.md](routes.md) shows how to forward to a payload
  behind a `Box`, an `Rc` or a reference, how to keep the set open with
  one `Box<dyn Trait>` variant, and how an arm reaches a field of the
  payload, or what a method returns.
- **What about methods only some variants handle, or the enum handles
  itself?**
  [inherent-methods.md](inherent-methods.md) forwards an inherent method
  to some variants, with a fallback for the rest.
  [partial-impl-blocks.md](partial-impl-blocks.md) lets you write some of
  a trait's methods by hand and forwards the rest.
- **Is the trait or the enum another crate's?**
  [restatements.md](restatements.md) covers traits from `std` and other
  crates, and enums from other crates, each restated once.
- **Is the enum behind a public struct?**
  [structs.md](structs.md) forwards a trait from a struct to one field.
- **What can it not do?**
  [limits.md](limits.md) has one line per limit, each with the fix where
  there is one.
- **I got an error. What does it mean?**
  [troubleshooting.md](troubleshooting.md) goes from the error message to
  its cause and the fix.
- **What does it need, and what does it add to my crate?**
  [compatibility.md](compatibility.md) covers `no_std`, the minimum Rust
  version, the dependency tree, and the carrier macro a `pub` trait
  exports.
- **Where is the API reference?**
  On [docs.rs](https://docs.rs/whichever). It carries the same material
  as these pages, with runnable examples. The tutorial, the glossary and
  troubleshooting are there too, as they were at the version you depend
  on.

## Choosing and comparing

- **Coming from `enum_dispatch`?**
  [moving-from-enum-dispatch.md](moving-from-enum-dispatch.md) is the
  swap, step by step and attribute for attribute, with the three things
  to watch.
- **How does it compare with the other crates?**
  [comparison.md](comparison.md) shows where `enum_dispatch` and the
  crates after it stop, with the full table probed against each.
- **When is an enum the right tool, and what does it cost?**
  [when-to-use-enum-dispatch.md](when-to-use-enum-dispatch.md) sets an
  enum against a trait object and a generic, with the measurement behind
  each piece of advice.
- **How fast, measured how?**
  [../benchmarks/README.md](../benchmarks/README.md) says what is
  compared with what and how, so you can trust and reproduce the numbers.
  [../benchmarks/RESULTS.md](../benchmarks/RESULTS.md) is the last run,
  with its machine, toolchain and date.

## How it works and why

- **What is this, and why is it shaped this way?**
  [vision.md](vision.md) is the argument, the five rules with what keeps
  each of them true, and what the crate will not be.
- **Why this and not the alternative?**
  [decisions/](decisions/) has one file per choice: the context, the
  decision, the evidence, what was not chosen, and what would reopen it.
- **What changed?**
  [../CHANGELOG.md](../CHANGELOG.md), one entry per release.
- **Who taught this crate what?**
  [../ACKNOWLEDGEMENTS.md](../ACKNOWLEDGEMENTS.md), one lesson per credit.

## Working on the crate

- **How is the macro crate built?**
  [../macros/ARCHITECTURE.md](../macros/ARCHITECTURE.md) shows how an
  expansion flows through the four proc macros, the module map, the
  forwarding engine, the protocol between the carriers, and the
  invariants every change keeps.
- **How are the combinations of features tested?**
  [../conformance/README.md](../conformance/README.md) covers the model,
  the cells it generates, how to read a run and settle a divergence, and
  the mutants that measure it.
  [decisions/combinations-are-checked-by-a-generated-matrix.md](decisions/combinations-are-checked-by-a-generated-matrix.md)
  explains why it is a generated matrix and not more hand-picked cases.
- **What are the project's rules and release steps?**
  [../CLAUDE.md](../CLAUDE.md) holds the rules of the project, and
  [../RELEASING.md](../RELEASING.md) the release checklist.
