# Benchmarks are runtime only

Status: accepted, 2026-11-01. Serves rule 5 of the [vision](../vision.md),
claims are measured against the hand. Reopen when the owner says so.

## Context

While the benchmark suite was built, measuring the compile time of the
competing crates was proposed alongside the runtime tables.

## Decision

The suite times the generated code and nothing else: one `&self` call per
element, each kind of method, and memory layout, from a crate other than
the enum's, without link-time optimisation. Compile time is not measured and
not quoted. The owner declined compile-time benchmarking of the competing
crates; the reason was not written down at the time, and this record keeps
the decision.

## Evidence

- `benchmarks/README.md` describes three suites, all runtime.
- The owner's decision, 2026-11-01.

## Not chosen

- A compile-time table beside the runtime ones.
