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
- The **verdict** is what the model says a cell is. A cell passes; or it is
  a limit answered by named sentences; or it is rejected, a forwarding no
  hand can write in Rust and no sentence covers, where the hand gets a
  named rustc error; or it is not Rust at all, which is malformed. A cell
  the model cannot write yet is unmodelled, and one whose axes that do not
  matter are not neutral is non-canonical; neither is ever run.
- The **outcome** is what a run observed for a cell. A cell whose outcome is
  not what its verdict allows has **diverged**.
- A **suite** is a selection of cells. A zone is the full product of a few
  axes. An array is a covering array within one class of verdict: every
  combination of values of any three axes among the passing cells, or of
  any two among the limits of one sentence and among the rejected cells,
  meets in some cell, the zones' and pinned cells counted. The pinned cells
  and the known divergences are suites of their own.

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
error of its own. A rejected cell holds the subject and the hand and no
law; it agrees when the hand fails with the named error, the subject fails
too, and no error of the subject sits on an attribute, which
`macros/ARCHITECTURE.md` 3.8 calls a defect. A malformed cell holds the
trait and the payloads.

The `callee` and `lift` axes are an inherent method's: whether the arm
calls the payload's method, of the same name or another, or reads the
target or a field of it, and what `with` wraps the result in.

The `path` axis says how `m` reaches the subject: the trait named on the
subject's attribute, a partial impl block (silent about `m`, `*`, naming
the first variant with a body for the rest, `default`, or writing `m`
itself), or an inherent block forwarding to the payloads' own `m`. The law
expects, per variant, what a twin of the payload answers, the body the
block writes, or what the trait's default answers.

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
`cargo check` runs first; a passing cell whose subject has an error is
checked again rendered with its hand alone, so that its hand's law still
runs and the divergence is known to be the macro's, and the check repeats
until the passing shards are clean; then `cargo test` runs their laws.
`--shape` expands every passing shard with `cargo expand` and compares the
operations of each forwarded method with its hand's; `--random N --seed S`
adds N random cells of the three classes for a soak run, outside the
census. `--whichever PATH` judges another checkout, an older release
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
  panic's first line;
- `shape:METHOD`, a forwarded method whose operations differ from the
  hand's under `--shape`; the message lists the differences.

An error whose span covers an attribute line of the cell is in the part
`attribute`, whatever part the line is in.

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

- `known.tsv` holds the known divergences, tab-separated: a pattern, a
  finding, and the reason, which is one sentence. A pattern is spelled as
  an id: the axes it names hold those values, a neutral value included
  when written out, and every other axis is free, so `base` matches every
  cell. A cell that diverges is excused when every finding that makes it
  diverge is excused by a line whose pattern matches it. Which findings
  make a cell diverge is decided per verdict in one place: for a pass,
  every finding; for a limit, every finding that is not a predicted
  sentence, and `missing:sentence` when none appears; for a rejected cell,
  `compiles:subject` and every finding of the subject that is neither a
  catalogued sentence nor a rustc error off the attributes; for a
  malformed cell, `compiles:trait`. A line names the axes that cause its
  defect, so that it cannot excuse another defect's finding. A line that
  excuses no finding in a complete run, every suite and no filter, fails it
  as stale; a line about a shape is judged so only under `--shape`. The
  pattern's own cell is always run.
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

## Findings the checks do not hold yet

The checks hold a sentence by its catalogue key, so a sentence with the
right key and the wrong words agrees. These were seen and are kept here
until a check holds them:

- A `Self` argument by value through a `deref` variant over `Rc` is
  reported as "`m` takes `self` by value", though the receiver is `&self`
  and the argument is what would move.
- `self: Pin<&mut Self>` and a `&mut Self` argument on a route ending in a
  method call are reported as "`m` takes `&mut self`".
- `self: Box<Self>` on a `deref` variant over `Rc` is reported as "holds a
  plain `Rc<P0>`; hold `Box<Rc<P0>>` in a `#[dispatch(deref)]` variant",
  which would dereference to `Rc<P0>`, no implementor of the trait.
- A reference in a sentence is spelled without its space, `&'aP0`.
