# Combinations are checked by a generated matrix

Status: accepted, 2026-11-14. Serves rules 1, 2 and 4 of the
[vision](../vision.md): the output is the hand's, the trait and the enum
may live anywhere, a limit is one sentence on the item. Reopen if a defect
inside the axes the matrix models is found by a survey while the matrix
agrees, which would say the model shares the macros' mistakes; if a full
run outgrows a CI job, where the arrays alone would have to stand for the
zones; or if the expansion can no longer be read on stable, which the
shape check needs.

## Context

The tests pinned points: one behaviour module per capability, one golden
per expansion worth reading, one compile-fail case per sentence. Every
defect the surveys of November 2026 found sat between those points, where
two or three features meet: a place under a lift, a `deref` variant under
`self: Box<Self>`, a mark under a `cfg`. The one time a product was
written out, 408 cells for a place handed over by `value`, sixty cells
failed where a hand's code compiled, and three defects were filed in a
day. The supported features multiply to more than a million million
combinations, so the question was how to test a space no one can write
out, without an expectation written by hand for each case.

## Decision

A crate of the workspace, `conformance/`, holds a model of what the
attributes support and judges the macros against it.

- A feature is an axis with its values: the receiver, the return type, how
  a variant reaches its target, the path a method takes to the subject. A
  cell is one value per axis, rendered as one small program.
- The model says what each cell is: it passes, it is a limit answered by
  named sentences, it is a forwarding no hand can write and rustc
  rejects, or it is not Rust. This table is `docs/limits.md` made
  executable, and each sentence in it names its `tests/ui` case.
- A passing cell holds the subject twice, spelled with the attributes and
  written by hand, and three checks need no expectation: the cell
  compiles without a diagnostic; forwarding commutes with the variant's
  constructor, so a method called on the subject answers what the same
  method called on the payload answers, on the attribute's subject and on
  the hand's; and the expansion performs the operations the hand performs,
  arm by arm, and nothing else.
- The hand is written from the documents and from Rust's semantics, never
  from the macro crate's sources.
- Cells are selected, not enumerated: the full product of a few axes
  where they are known to interact, and covering arrays in which every
  three values of the core axes and every two values of any axes meet in
  some passing cell.
- A defect the matrix finds and nobody has fixed is a line of
  `conformance/known.tsv`, which fails once the defect is gone. The number
  of cells per suite and verdict is pinned in `conformance/census.txt`, so
  a rule that silently empties a suite shows as a diff.
- The matrix is itself measured: `conformance/mutants/` holds small
  changes to the macros, each injected alone, each recorded as noticed or
  not with the reason.

The hand-picked tests stay. `tests/ui` pins the words of a sentence, the
goldens are read by eye, the behaviour modules follow the README's
claims, and the smoke crates hold the two-version graph; the matrix holds
the space between them.

## Evidence

- `conformance/README.md` is how the matrix is read, run and extended;
  `conformance/src/lib.rs` is its architecture.
- At this record: 34,744 cells over fourteen axes, about two and a half
  minutes on a laptop; 19,087 pass, 14,125 are limits, 1,502 are
  rejected.
- In its first two days it found eight defects no survey had: a rustc
  error in an arm landing on an attribute, a warning from generated code
  in a partial block, a cascade beside a sentence, a sentence about a
  variant that its `cfg` compiled out, a mark under a trait method's own
  `cfg` reported wider than first filed, an option naming a method a
  `cfg` compiled out, a projection qualified with its bound's
  constraints, and three sentences naming a receiver the method does not
  take. Each is a line of `conformance/known.tsv` until it is fixed.
- The cells of the defects the surveys had found are pinned in
  `conformance/pinned.txt`, and reproduce against the releases that had
  them.
- The trait in another crate agrees under every axis the arrays reach.

## Not chosen

- More hand-picked cases. Each defect above needed two or three features
  to meet, and nobody picks the pairs that fail.
- A snapshot per combination, as the goldens and `tests/ui` are. A
  snapshot says the output is what it was, not that it is right, and
  thirty thousand of them are read by no one.
- Token equality with the hand. It pins spelling that carries no meaning,
  `&**inner` against `inner`, and every such difference would have to be
  triaged; the operations are compared instead, and the type checker and
  the law hold the rest.
- Random token streams fed to the macros. Without a model there is no
  saying what the output should be, so only a crash is a finding.
- Expanding in process, without rustc. The defects were type errors and
  borrows, which only the compiler sees.
- A checked-in line per cell. Selection moves with the model, so the file
  would churn on every change; the census and the known lines carry what
  a reviewer has to see.
