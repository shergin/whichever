# Working in this repository

Read `notes/README.md` first when it exists in your checkout; the `notes/`
tree is private and git-invisible, and a fresh clone does not have it. The
public argument and the rules are `docs/vision.md`, the choices are
`docs/decisions/`, and the macro crate's architecture, with the invariants
every change to it keeps, is `macros/ARCHITECTURE.md`. These files outrank
any default.

## The rules of the project

- The output is the hand's. The attribute writes one `match` per method, a
  fully qualified call per arm, `#[inline]`, and nothing a careful hand
  would not write: no trait object, no closure, no helper trait, no
  allocation, no refcount, no conversion the signature does not ask for.
- Nothing is kept between expansions. The trait's tokens travel in its
  carrier macro; no cache, registry or file outlives an expansion.
- A shape earns support when a surveyed project writes the forwarding by
  hand and no composition of what exists expresses it. A shape without a
  motivating case goes on the watch list in `notes/PLAN.md` with a trigger,
  not into the crate.
- A limit is one sentence on the offending item, naming the way around it
  where one exists, with the enum still emitted. Every limit has a
  `tests/ui` case and a line in `docs/limits.md`, landed together.
- Claims are measured against the hand. A number in the docs comes from
  `benchmarks/RESULTS.md`, with machine, toolchain and date; the reference
  row is the hand-written `match`. Benchmarks are runtime only.
- The README is the case for the crate: what the attributes write, where
  the other crates stop in a few lines each, and the measurements. The
  comparison in full is `docs/comparison.md`; how an attribute is used, an
  option or a limit, goes on its page under `docs/` and in the crate docs,
  and the README links there.
- The README and the pages under `docs/` name a concept before code does,
  with one word per thing: the macro `#[dispatchable]` leaves next to a
  trait is the carrier macro. The macro source's internal names, such as
  "subject" for the enum or struct, stay out of public text.
- Public docs are written for a reader new to the crate, in plain English:
  short sentences that speak to "you", the everyday case and a small
  example first, the fine print after it under a heading of its own, and
  the reason beside each rule. Every fact stays: an option, an exception
  or a limit is never left out to make a page lighter. The words are the
  ones `docs/glossary.md` defines.
- Public docs are updated in the same change that makes them stale: the
  README, the pages under `docs/`, the crate docs in `src/lib.rs`, the
  Unreleased section of `CHANGELOG.md`, and the guide when a measurement
  it quotes moves. `docs/tutorial.md` shows code from `examples/tour.rs`,
  and the two change together.
- Three pages are in the crate docs as well, through `src/docs.rs`: the
  tutorial, the glossary and troubleshooting. Their links to files are
  reference-style, and a label a page defines gets a line in
  `src/docs.rs` that says where it goes on docs.rs.
- Settled stays settled. A choice among real alternatives is recorded in
  `docs/decisions/` with the condition that reopens it. Do not relitigate
  without new evidence; when evidence arrives, supersede the record rather
  than edit it.

## Surveys and gap notes

- A survey follows `notes/oss-fit-research.md`: one fresh scratch directory
  per download, read the source, never run the surveyed project's build or
  tests. Probes are small crates of our own, compiled in scratch.
- A shape `whichever` cannot reach is a result: a note under `notes/gaps/`
  with the motivating project at a named version, an acceptance test and
  the directions, and a line in the survey log.

## Tests

- `tests/ui/` holds one compile-fail case per limit, with the `.stderr`
  pinned to stable's wording. `tests/expand/` holds the expansion goldens,
  run with `cargo test --test expand -- --ignored` under `cargo-expand`.
  `tests/behaviour/` runs the generated code, one module per capability. `smoke/` holds the
  cross-crate, renamed-dependency and `no_std` crates, and `smoke/mixed` the
  two-version graph an older reader has to report.
  `benchmarks/subjects` checks every subject against the hand before
  anything is timed. The macro crate's `options`, `protocol` and `shape`
  modules carry unit tests of their own.
- `conformance/` holds the conformance matrix: a model of the features the
  attributes support, one generated cell per selected combination, each
  judged against a hand-written twin, run with
  `cargo run -p whichever-conformance --release -- run --shape` under
  `cargo-expand`. The hand in a cell is written from the documents, never
  from the macro crate's sources. A divergence is settled by correcting the
  model, by adding a limit to its table with its `tests/ui` case, or, when
  the macros are wrong, by a line of `conformance/known.tsv`. A fix of the
  macros deletes the `known.tsv` lines it names and adds a mutant that
  reverts it, with a `witness:` line naming a cell that agrees without the
  mutant and diverges with it.
- A `.stderr` or `.expanded.rs` is regenerated by its tool, and
  `conformance/census.txt` by `census --bless`, which runs no cell; none
  is edited by hand to fit the code.
- Production code comes first; tests serve it. When a test stands in the
  way of the right expansion, the test changes.

## Git and releases

- One meaningful change per commit; imperative subject under about fifty
  characters; a body only when the diff cannot speak for itself. No
  attribution lines, trailers or session links in commits or pull requests.
- A release commit is `Release <version>` and touches the manifests and the
  changelog heading; `RELEASING.md` is the checklist. Every tag `v<version>`
  becomes a GitHub release through `.github/workflows/release.yml`, named
  after the version, with the opening paragraph of the changelog entry as
  its notes, which `scripts/release_notes.py` prints.
- Run what CI runs before a commit:
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`,
  `cargo test --test expand -- --ignored`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p whichever -p whichever-macros`,
  `python3 scripts/check_doc_links.py`, which reads what that built,
  `cargo check -p whichever --no-default-features --tests`,
  `cargo check -p smoke-no-std --target thumbv7em-none-eabi`,
  `python3 smoke/mixed/check.py`,
  `cargo bench -p whichever-benchmarks --no-run`,
  `python3 benchmarks/scripts/codegen.py`,
  `python3 benchmarks/scripts/check_numbers.py`,
  `python3 scripts/check_links.py`, and
  `cargo run -p whichever-conformance --release -- run --shape`.
  The mutants are judged by their witnesses, the cells each mutant's file
  names: `.github/workflows/mutants.yml` runs them in four jobs on every
  push to `main` and every pull request that touches `macros/src/` or
  `conformance/`, and weekly, and `mutants --only NAME` is the local
  check, seconds a mutant. `mutants --full` runs the matrix once per
  mutant, hours over all of them: it is for finding a witness and for
  measuring how widely a mutant is noticed, and CI runs it only when
  started by hand.

## Rust

- Stable toolchain, minimum supported version 1.88. `cargo fmt` before
  `cargo check`; clippy clean. `whichever` is `#![no_std]`,
  `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`.
- The macro crate's dependencies are `proc-macro2`, `quote`, `syn` and
  `proc-macro-crate`, and that is the whole tree. A dependency in
  `whichever` itself, or a new one in the macro crate, is a decision.
- Generated code names only `core` items, fully qualified.
- Prefer early returns. Name things with full words; the accepted
  abbreviations are the idiomatic ones and the ones `syn` uses (`ident`,
  `attr`, `ty`).
- Comments explain non-obvious logic, a design choice or a component, not
  the code. Full English sentences ending with a period; ASCII only;
  identifiers in backticks; no decorative characters. Doc comments on
  public items are noun phrases for types and third-person sentences for
  functions.
