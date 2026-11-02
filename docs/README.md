# Docs

What to read when.

- **What is this, and why is it shaped this way?**
  [vision.md](vision.md) is the argument, the five rules with what holds
  each of them, and what the crate will not be.
- **Why this and not the alternative?**
  [decisions/](decisions/) has one file per choice: the context, the
  decision, the evidence, what was not chosen, and what would reopen it.
- **What do the attributes generate, and what are the options and limits?**
  The [README](../README.md), and the API reference on
  [docs.rs](https://docs.rs/whichever), which carries the same text with
  runnable examples.
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
