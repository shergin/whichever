# The dispatch collection outlasts the predictor

Status: accepted, 2026-11-03. Serves rule 5 of the [vision](../vision.md),
claims are measured against the hand. Reopen when a machine the suite is run
on shows the `shuffled` rows still falling as the collection grows past
64 Ki elements, or when the `uniform` rows show the walk through L2 costing
more than one inside L1.

## Context

The `dispatch` and `methods` benchmarks walked 1024 elements, sized to fit
in L1 so that memory stayed out of the measurement. The loop replays the
same sequence of variants on every iteration, and a branch predictor with a
long history learns a random sequence of that length, so the `shuffled`
layout, documented as the one where the branch is unpredictable, measured a
memorised sequence instead at 16 and 32 variants and with out-of-line
payload methods.

## Decision

The collection is 64 Ki elements, 1.5 MiB of enums. The walk is
sequential, so the prefetcher keeps it as cheap per element as a walk
inside L1, and the random layout costs within a few percent of what it does
over a million elements. `memory/build_and_drop` keeps its 1024 elements,
since it times allocation and not dispatch.

`memory/scattered`, which grows the collection from 1 Ki to 4 Mi elements on
purpose, holds one variant throughout. With random variants, the predictor
learned its smallest collection and not the others, and the difference read
as the cost of scattered boxes: at 16 Ki elements, 6.71 ns against 6.79 ns
for boxes allocated in order over 64 Ki, in the full run of 2026-11-03
that preceded this change to the group.

## Evidence

A probe over the suite's own subjects and loop, timing `sum_area` from a
crate of its own, the best of nine timings in each of two runs (up to 4 Ki
from the first, the rest from the second), on an Apple M1 Pro with rustc 1.98.0,
2026-11-03, in nanoseconds per element (`hand`, `shuffled` layout unless
named):

| | 1 Ki | 4 Ki | 16 Ki | 64 Ki | 1 Mi |
| --- | ---: | ---: | ---: | ---: | ---: |
| `trivial`, 32 variants | 1.56 | 5.96 | 7.23 | 7.26 | 7.32 |
| `opaque`, 8 variants | 2.07 | 2.26 | 7.60 | 8.81 | 8.99 |
| `opaque`, 8 variants, `uniform` | 0.97 | 0.97 | 0.95 | 0.95 | 0.96 |

The rise starts while 4 Ki elements still fit in L1, and the `uniform` row
does not move, so it is the predictor and not memory.

## Not chosen

- 1024 elements with a fresh random sequence for every iteration: building
  it would be timed with the loop, or the loop would have to be timed over
  a pool of sequences, which is a longer collection by another name.
- A longer collection for `shuffled` only: the three layouts would no
  longer differ in one variable.
- 256 Ki elements or more: no closer to the limit, and the boxed collection
  of the `Box<dyn>` row would approach the size of L2.
