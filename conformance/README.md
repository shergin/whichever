# The conformance matrix

The tests of the facade pin points: one behaviour module per capability,
one golden per expansion worth reading, one compile-fail case per sentence.
The defects the surveys found sat between those points, where two or three
features meet. The conformance matrix tests the space between them. It
holds a model of what the attributes support, generates one small program
per combination of features, and judges every program by checks that need
no expectation written by hand.

This crate is never published and nothing in the facade depends on it.

## The words

- An **axis** is one feature with its values: the receiver of a method, its
  return type, how a variant reaches its target. The first value of an axis
  is its neutral value.
- A **cell** is one value per axis. Its **id** spells the axes whose value
  is not neutral, as `name=word` pairs in axis order, such as
  `s0=deref-box recv=box ret=self`; the neutral cell is `base`. An axis
  added later is neutral in every id written before it, so an id stays
  valid.
- The **verdict** is what the model says a cell is. A cell passes, or it is
  a limit answered by named sentences, or it is not Rust at all, which is
  malformed. A cell the model cannot write yet is unmodelled, and one whose
  axes that do not matter are not neutral is non-canonical; neither is ever
  run.
- The **outcome** is what a run observed for a cell. A cell whose outcome is
  not what its verdict allows has **diverged**.
- A **suite** is a selection of cells. A zone is the full product of a few
  axes; the pinned cells and the known divergences are suites of their own.

`cargo run -p whichever-conformance --release -- axes` lists the axes and
their values.

## What a cell holds

A cell is one source file. Its header names the cell and its verdict. A
passing cell holds:

1. The trait, `Probe`, with `#[whichever::dispatchable]`: the method under
   test, `m`, and a control method, `base`, that every subject forwards.
2. The payloads `P0` and `P1`, whose answers say which payload gave them,
   from which state and for which arguments; a method taking `&mut self`
   changes the state. Each payload also has an inherent method named `m`
   that answers a poison value, so an arm that calls it instead of the
   trait's method shows in the law.
3. The subject, `E`, spelled with `#[whichever::dispatch(..)]`.
4. `mod hand`, the same enum with the forwarding a careful hand writes, as
   `docs/what-is-generated.md` and the goldens show it. The hand is written
   from the documents and from Rust's semantics, never from the macro
   crate's sources: a model that mirrors the implementation agrees with its
   defects.
5. `mod law`, two tests with one body, `subject` against `E` and `hand`
   against `hand::E`. For every variant and argument, the method called on
   the subject answers what it answers on a twin of the payload, converted
   as the return type asks, and leaves the payload in the twin's state.

A limit cell holds the trait, the payloads, the subject and a probe that
calls `base` through the trait, so a withheld impl or a cascade shows as an
error of its own. A malformed cell holds the trait and the payloads.

## Running it

```sh
cargo run -p whichever-conformance --release -- run
cargo run -p whichever-conformance --release -- run --suite boxed --filter recv=box
cargo run -p whichever-conformance --release -- run --whichever ../another-checkout
cargo run -p whichever-conformance --release -- list --suite pinned
cargo run -p whichever-conformance --release -- show 's0=via-field recv=mut'
```

A run writes a workspace of shard crates under `target/conformance/ws`, one
file per cell, and builds it in `target/conformance/target`. A file is
written only when it changed, so a second run rebuilds only what moved.
`cargo check` runs first; a passing cell with an error is set aside and the
check repeats until the passing shards are clean, then `cargo test` runs
their laws. `--whichever PATH` judges another checkout, an older release
exported with `git archive` or a mutated copy, with the same model. `--jobs
N` is handed to cargo.

The run exits 0 when nothing diverged and the ledger agrees, 1 on a
divergence or a ledger that disagrees, and 2 on a usage error, a harness
error, or a hand that does not compile or fails its own law. The full
table is written to `target/conformance/report.tsv`.

## Reading a divergence

A divergence is printed like this one, which a forwarding that called the
payload's inherent `m` would give:

```text
diverges: `s0=deref-box recv=pin-mut`
  note      diverged
  verdict   pass
  outcome   law:subject
  message   law subject: assertion `left == right` failed: m on A with x = 2
  file      target/conformance/ws/pass-003/src/c_5f3a2b1c0d9e.rs
  show      cargo run -p whichever-conformance --release -- show 's0=deref-box recv=pin-mut'
```

An outcome is `pass` or a list of findings:

- `sentence:KEY@PLACE`, a sentence of the macro by its catalogue key, or by
  its text in quotes when the catalogue does not hold it, and where it was
  reported: an anchor such as `method` or `variant-1`, or `line-N`;
- `error:CODE@PART`, a rustc error, by code, in a part of the cell:
  `trait`, `payloads`, `subject`, `probe`, `hand`, `law-subject` or
  `law-hand`;
- `warning:LINT@PART`, a warning;
- `law:subject` or `law:hand`, a failed test of the law; the message is the
  panic's first line.

The file is the cell as it was compiled; `show` prints it again from the
model. A finding in the hand, in its law, or in the payloads blames the
model and is reported as `model-broken`.

## Settling a divergence

Every divergence is settled in exactly one of three ways.

1. **The model was wrong**: the hand, the payloads, the law, a verdict rule.
   Fix the model. A hand that does not compile or fails its law is always
   this.
2. **A limit was missing from the table**: the outcome is a sentence that
   `docs/limits.md` or a `tests/ui` case documents. Add the rule to
   `src/model/verdict.rs`, quoting the document, and the sentence to the
   catalogue in `src/model/limits.rs` with the case that pins it.
3. **The macro is wrong**: a rustc error in generated code where the hand
   compiles, a wrong answer, a sentence on a shape the documents say is
   supported, a cascade, a warning from generated code, or a limit the
   documents promise that does not appear. Reduce it to a standalone crate,
   then add the cell to `known.tsv` with the outcome the run printed and one
   sentence saying what is wrong. The hand and the verdict stay as the
   documents say.

Where the documents are silent or ambiguous, a rule takes the reading the
documents and the goldens best support and says so in a comment beginning
"Open question".

## The ledger

- `known.tsv` holds the known divergences, tab-separated: the id, the
  outcome, and the reason, which is one sentence. A listed divergence with
  the same outcome does not fail the run; a listed cell that agrees, or
  diverges otherwise, does, and says which line to update. Every listed cell
  is always run.
- `pinned.txt` holds cells always run, one id per line, with a `#` comment
  naming where the shape came from.
- `census.txt` counts, for every suite, the cells by verdict and the
  combinations never selected by reason. A run compares it exactly, so a
  rule that silently empties a suite shows as a difference. `run --bless`
  rewrites it, and never touches `known.tsv`.

## Adding a value to an axis

An axis is an enum declared with `axis!` in `src/model/axes.rs`, one
documented `Value = "word"` line per value. A value describes itself through
its descriptor, in the module of its kind: a target kind through its
storage, its form and the steps of its route (`src/model/target.rs`); a
receiver through how the enum is unwrapped and the target handed over
(`src/model/receiver.rs`); a return type through a small tree of nodes
(`src/model/returns.rs`). The renderers interpret descriptors, so a new
value is one variant and its descriptor. Every `match` over an axis or a
descriptor is exhaustive, so the compiler names every place that needs to
know about the new value, the verdict among them. Then run the suite, settle
what diverges, and bless the census.

A new axis is one more line in the `cell!` invocation in `src/cell.rs`, its
neutral value first, so no id changes. A new zone is one more entry in
`src/select/zones.rs`.
