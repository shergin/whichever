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
2. The payloads `P0`, `P1` and `P2`, whose answers say which payload gave them,
   from which state and for which arguments; a method taking `&mut self`
   changes the state, and one answering `()`, which shows nothing, notes
   the number its answer would be made from in a `core::cell::Cell` of the
   support module, which the law reads back after the call. Each payload
   also has an inherent method named `m` that answers a poison value, so
   an arm that calls it instead of the trait's method shows in the law.
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
law; it agrees when the hand fails with the named error and the subject
fails with the same code and with nothing the hand does not get: a
sentence, which no rejected cell predicts, or an error of a code the hand
does not get, a cascade, diverges, and so does an error of the subject on
an attribute, which
`macros/ARCHITECTURE.md` 3.8 calls a defect. A malformed cell holds the
trait and the payloads.

The `callee` and `lift` axes are an inherent method's: whether the arm
calls the payload's method, of the same name or another, or reads the
target or a field of it, and what `with` wraps the result in. An inherent
method's `Self` argument is the enum, handed over as it is, which the
payload's method takes as any implementor of the trait and whose control
method its answer reads, so the law calls it with the argument holding the
same variant and the next: an arm that handed over a payload instead shows
there. On a struct it is the other value's field. Under `value` the two variants hold one payload
type, each reaching it as its target kind does. `with = Into::into` names
the enum in the return type and rebuilds it through the `From` impls the
attribute writes, which the hand writes for its own enum.

The `item` axis is the attributes on the trait's method: a `#[cfg]` that
holds or does not, `#[deprecated]`, an `#[allow]` of a lint its signature
trips, and its own `#[inline(..)]`; and `sized`, the bound `where Self:
Sized`, which a trait object does not meet. Every impl that repeats the
signature, a block's mark on `m` included, repeats the bound and carries
the method's `#[cfg]` and its `#[allow]`, as a hand writes them. A method
under a `#[cfg]` that does not hold exists in no build: its sentences and
rejections are dropped, the hand writes none of it, and the law checks the
control method alone.

A boxed future is spelled as `#[async_trait]` writes it, as the golden
`boxed_futures` does: `Pin<Box<dyn Future<Output = T> + Send +
'async_trait>>`, the receiver's reference named `'life0` and a `Self`
argument's `'life1`, each bound to outlive the future, as are `Self` and
the method's type parameters, a parameter the converting block holds
declared `Send`.

Three values of `s0`, `struct`, `struct-marked` and `struct-named`, make
the subject a struct: one field, a field marked `#[dispatch]` beside a
label, or one named field, which the hand rebuilds by its braces and
which a `Self` under an `Option` or a `Result` cannot be rebuilt into. A struct
has no second variant, so `s1` stays `none`, and its one arm hands a
return-position `impl Trait` or a future over as it is, as
`docs/structs.md` says.

Four values of `s0` and `s1` reach their target by a route of several
steps, which `docs/routes.md` allows in any sequence: `via = *.payload`
behind a `Box` and behind an `Rc`, `via = .1.payload` through a tuple
holding a wrapper, and `via = .get().payload`, a field of what a method
lends, which a hand can neither borrow mutably nor move out of.

The `s1` value `deref-dyn` holds the second payload as a trait object,
`#[dispatch(deref)] B(Box<dyn Probe>)`, the open set beside the sized
payloads of `docs/routes.md`. The law builds it from a fresh `P1` and keeps
the twin concrete; the hand names the target `dyn Probe`, and `seen` reads
the payload's state through the control method, the one thing a trait
object shows. The trait stays dyn-compatible beside it, so whatever would
make it not, a `Self` in a signature, an `impl Trait` return, a generic or
`async` method, an associated item, a generic or second trait, is
non-canonical there, and so are the conversions, an inherent method and
another crate's enum or trait. A method that requires `Self: Sized`, the
`item` value `sized` or a default taking `self`, is the sentence of
`dyn_payload_under_sized_method` on that variant's arm, and a block's
`#[dispatch(default)]` leaves it to the trait's default; under `self` or
a pinned receiver, the trait object is rustc's E0277 for the hand as well.

Two values of `ret` return a generic associated type, `Self::Item<'_>`,
declared `type Item<'v> where Self: 'v;`, its parameter named apart from
the `'a` of a subject that borrows. Every payload gives it a loan of its
state, `support::Loan0<'v>`, which converts into the
`support::View<'v>` the enum declares, `type Item<'v> = support::View<'v>
where Self: 'v;`, as `docs/partial-impl-blocks.md` and the golden
`gat_in_a_partial_block` write it. Under `gat` a partial block declares it
where the cell has one; under `gat-undeclared` the block leaves it out.
Without the declaration, on the enum's attribute or in such a block, it is
the sentence of `generic_associated_type` and `gat_undeclared_in_block`
on the trait's declaration.

The `s2` axis is a third variant, `C` over `P2`, beside a second: a
plain payload, a `Box` under `deref`, a field of a wrapper, or a unit
variant. With it a `match` has three arms, a mismatch is checked between
each variant and the next and between the last and the first, an
associated item is checked across three payloads, and in a block the
control method names every variant that holds a payload, with a body for
the rest.

The `decl` axis says how the trait is declared (`docs/restatements.md`).
Under `remote` the trait is declared without the attribute, in a module
`real` of the cell, in the sibling module or in the trait's crate, and the
subject's crate restates it with `#[whichever::dispatchable(remote =
path)]`, which the subject's attribute reads; under `remote-base` the
restatement lists the control method alone, so a provided `m` is the real
trait's default whatever its payload overrides. Under `foreign` the trait
is `whichever::foreign::Display`: the payloads implement
`core::fmt::Display`, beside an inherent `fmt` that prints a poison, the
hand implements `core::fmt::Display` with the same `match`, and the law
compares what the subject prints with what a twin of its payload prints.
Under `unsafe` the trait is an `unsafe trait`, and every impl of it, the
payloads', the hand's and a block's, is an `unsafe impl`.

The `owner` axis says which crate declares the enum. Under `remote` the
trait's crate shard declares `E` generic over what its variants hold, as
another crate declares `either::Either`, and the subject's crate restates
it with `#[whichever::dispatch(remote = path)]` beside the trait's
attribute; the impl is written for the real path, with the bound `Tk:
Probe` per payload parameter, the hand is the same generic enum with that
bound, and the law takes both at the payloads, `type S = E<P0, P1>;`. A
conversion option on a restated enum is its sentence.

The `path` axis says how `m` reaches the subject: the trait named on the
subject's attribute, a partial impl block (silent about `m`, `*`, naming
the first variant with a body for the rest, `default`, or writing `m`
itself), or an inherent block forwarding to the payloads' own `m`. The law
expects, per variant, what a twin of the payload answers, the body the
block writes, or what the trait's default answers.

The `home` axis says where the trait and the payloads live: in the cell's
module, in a sibling module the subject imports the trait from, or in
another crate. There the cell is written as two files that keep its line
numbers: the trait's crate, a `traits-NNN` shard every other shard depends
on, holds the cell up to its payloads, and the subject's shard holds the
rest and takes `support` from the trait's crate. A cell whose trait's crate
fails is set aside from both, its findings kept, and the check repeats. A
trait in a sibling module names its argument `crate::support::Count`, and
one in another crate names `crate::Tally`, a type only the trait's shard
declares at its root, so the rewrite of `crate::` paths to the trait's
crate is exercised: left as written, the path names nothing in the
subject's crate.

The `generic` axis is a payload that is the enum's type parameter, a trait
generic over a format dispatched at `support::Json`, a trait generic over
a const dispatched at the enum's own parameter, a defaulted parameter
left out, a trait whose argument is `F::Pair<F>`, a projection whose
later segment has generic arguments of its own, and a trait with a
lifetime parameter the enum names, `Probe<'static>`. The law names a
generic subject by an alias at the argument it takes it at, `type S =
E<P0>;`, and builds and takes apart a generic tuple struct by its path,
`Built::<P0>(..)`, which an alias cannot spell.

Two more values of `generic` are a partial block's own header
(`docs/partial-impl-blocks.md`, "The impl header"). Under `every` the
trait is generic over a format and the block implements it for every
`F`, `impl<F: support::Format<Value = u64>> Probe<F> for E {}`, with the
bound the payloads' impls take so that their answers count the argument;
under `instance` the enum is generic over its first payload, `enum E<T> {
A(T), .. }`, and the block implements the trait for the instantiation the
law takes, `impl Probe for E<P0> {}`, whose arms name `P0`. The enum's own
options, `from = false` beside the parameter, stay on its attribute. Both
are a block's alone.

The `conv` axis checks the conversions the subject's attribute writes:
`From` per payload, `from = false`, the three `try_from` forms, `into =
support::Sink`, and the variant's own `from = false` and `try_from = true`.
The law builds the enum through each `From` and takes each payload back
through `TryFrom`, its own variant answering the payload and another
handing the enum back; both convert what a variant of one field stores, a
`deref` variant's pointer included, as the golden `inherent` writes
`TryFrom<AnyShape> for Box<Square>`. An impl the docs say is not written,
beside a variant with other fields, on a struct or under `from = false`, is
written by hand beside the subject, which compiles only if the attribute
wrote none. A payload that is the enum's type parameter takes the
conversions too: its `From` beside another variant's, and its `TryFrom`,
are each a sentence on the variant.

The `agree` axis gives the trait associated items of its own beside those
the return types name: a type the payloads give differently, consts of
every checked type that agree or of which one differs, a const of a type
the check does not compare, and consts fixed on the enum with `NAME =
value`. A disagreement is a limit on the variant that disagrees with the
supplier, the first variant without a `#[cfg]` whose target has a type; a
variant reached through a field or a method takes no part, and a const on
an enum with a type or const parameter is not checked; a lifetime is no
matter. The hand takes each
item from the supplier or as fixed, and the law reads every item through
the subject. Under `type-returned` the type that differs is returned by a
method every subject forwards as it forwards the control method, `fn
kind(&self) -> Self::Kind`, whose arms are rustc's E0308 on the enum where
two of them differ, beside the check's E0271 or alone where the variant
takes no part in the check.

The `stack` axis dispatches a second trait, `Other`, a subtrait `Other:
Probe`, or a supertrait the trait under test requires, `Probe: Other`, on
the same subject through an attribute of its own. Its
one method takes the control method's receiver and is checked by the law
as `base` is; it declares the associated items the `agree` axis gives
`Probe` and takes the same overrides. In a block path the subject's
attribute names `Other` and the block is the impl of `Probe`. The
`deprecated` axis puts `#[deprecated]` on a variant: the hand writes its
arms and the conversions that name it under `#[allow(deprecated)]`, and
the law, which names it, allows the lint.

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
The nested cargo runs with none of the outer cargo's environment but
`CARGO_HOME` and `CARGO_INCREMENTAL`: a run that starts cold, as CI's does,
sets `CARGO_INCREMENTAL=0`, since an incremental cache no later run reads
only fills the disk.
`cargo check` runs first; a passing cell whose subject has an error is
checked again rendered with its hand alone, so that its hand's law still
runs and the divergence is known to be the macro's, and the check repeats
until the passing shards are clean; then `cargo test` runs their laws.
`--shape` expands every passing shard with `cargo expand` and compares the
operations of each forwarded method with its hand's, and its attributes:
`#[inline]` or the trait method's own, what the trait method passes on, and
nothing on an inherent method the user did not write; and that the impl
the subject's attribute writes is `#[automatically_derived]`; `--random N --seed S`
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
- `wording:KEY`, a predicted sentence that lacks what the cell asks of it:
  the way out its catalogue entry names, as its `tests/ui` snapshot spells
  it; the method's own way out where the sentence ends in one, by hand,
  a body, or leaving the variant to the body the method has; and the names
  it interpolates, the method's, the variant's, what the method takes as
  the signature spells it, and the payload's type as written;
- `error:CODE@PART`, a rustc error, by code, in a part of the cell:
  `trait`, `payloads`, `subject`, `probe`, `hand`, `law-subject` or
  `law-hand`;
- `warning:LINT@PART`, a warning;
- `law:subject` or `law:hand`, a failed test of the law; the message is the
  panic's first line;
- `shape:METHOD`, a forwarded method whose operations or attributes
  differ from the hand's under `--shape`, or `shape:impl`, an impl the
  subject's attribute writes without `#[automatically_derived]`; the
  message lists the differences.

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
  sentence, `missing:sentence` when none appears, and `missing:KEY@ANCHOR`
  for a predicted sentence about the subject that does not, a variant's
  shape, an agreement or a conversion option, and for an anchor none of
  whose predicted sentences of the method does, since an arm or a method
  carries one of them; for a rejected cell,
  `compiles:subject`, `missing:error:CODE@subject` when the subject fails
  without the code the model names, and every finding of the subject that
  is not an error off the attributes of a code the hand gets too; for a
  malformed cell, `compiles:trait`. A line names the axes that cause its
  defect, so that it cannot excuse another defect's finding. A line that
  excuses no finding in a complete run, every suite and no filter, fails it
  as stale; a line about a shape is judged so only under `--shape`. The
  pattern's own cell is always run when the model selects it, and a line
  is judged stale whether or not it does.
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

## The mutants

A matrix that agrees says nothing of what it would notice. A **mutant**
is one small change to the crate under test that a developer could make
by mistake and that leaves the macros compiling: a borrow dropped in an
arm, a limit no longer reported, an `#[inline]` lost. The mutants measure
the matrix: each is injected alone, and the matrix either notices it or
does not.

A mutant is a file of `mutants/`, `<name>.mutant`:

```text
file: macros/src/forwarding.rs
breaks: The arms of an `async fn` are awaited, as `docs/what-is-generated.md` says.
expect: killed

find:
            value = quote_spanned! { site => #value.await };
replace:
            value = quote_spanned! { site => #value };
```

`file` is the source file it changes, relative to the repository's root;
`breaks` is the promise it breaks and the document that makes it;
`expect` is `killed`, or `survives` with a line `reason:` saying in one
sentence what the matrix lacks. The lines between `find:` and `replace:`
are the text to find, which occurs exactly once in the file, and the
lines after `replace:` replace it. A mutant whose text no longer occurs
exactly once is stale: the macros moved on, and the mutant is updated or
deleted rather than left to mutate something else. A unit test of this
crate checks that every mutant still applies.

```sh
cargo run -p whichever-conformance --release -- mutants
cargo run -p whichever-conformance --release -- mutants --only await-dropped
cargo run -p whichever-conformance --release -- mutants --suite array --suite pinned
```

The command runs the matrix with `--shape` on the unmodified crate, and
stops with a harness error unless that run agrees. Then, per mutant, it
copies the crate under test, the facade's manifest and sources and the
macro crate's, to `target/conformance/mutants/<name>`, applies the
change, checks that the copy compiles, and runs the matrix against the
copy as `run --shape --whichever PATH` does. `--suite` narrows both runs
as it narrows `run`, and `--no-shape` leaves the shape out of both. A
full run per mutant builds every shard again, about as long as a cold
run.

A mutant is **killed** when a cell that agreed in the baseline diverges,
or a line of `known.tsv` goes stale; otherwise it **survived**. The table
is printed and written to `target/conformance/mutants.tsv`, one line per
mutant:

- `result`: `killed` or `survived`; `stale` for a text that does not
  occur once, `uncompiled` for a copy that does not compile, and
  `harness` for a run that could not judge, which count as neither;
- `expected`: what the mutant's `expect:` line says;
- `checks`: what caught it: `verdict` for an error, a warning or a
  sentence where the verdict allows none, or a sentence missing; `law`;
  `shape`; `stale` for a known line that excuses nothing; `hand` for a
  hand that broke, where the mutant changed what the trait's attribute
  writes;
- `cells`: how many cells newly diverge;
- `example`: one of them, with its first new finding and its message, or
  the stale line. For a survivor, the cells that agreed in the baseline
  and now diverge where a line of `known.tsv` excuses them, if any: a line
  wider than its defect hides a mutant there.

Below the table come the kill rate among the mutants that compiled and
ran, and every mutant whose result is not its `expect:` line. The command
exits 0 when every result is what its mutant expects; 1 when a mutant
expected to be killed survived, which means the matrix lost a check, or
one expected to survive was killed, whose `expect:` line is then updated
as a stale `known.tsv` line is removed; and 2 on a usage or harness
error, a stale or uncompiled mutant included.

A mutant expected to survive is a gap of the matrix with its reason, and
the reason says what would close it: an axis value, a zone, a check. When
the matrix grows to close it, the run says so and the line becomes
`expect: killed`. A defect the matrix should have caught becomes a mutant
when it is fixed: the fix reverted is the mutant, so the matrix keeps
proving that it would notice the defect again.
