# Releasing whichever

Releases are cut from a clean `main` with every CI job green. Two crates
ship together at one version, `whichever-macros` and `whichever`, and
`whichever` pins the macro crate to the exact version, since `__private`
and the generated code are one interface.

## 1. Prepare the release commit

- Choose the version. Before 1.0 a breaking change raises the minor
  version, and the changelog lists it without apology.
- Set `version` in `Cargo.toml` and in `macros/Cargo.toml` to the same
  number, and the `whichever-macros` dependency in `Cargo.toml` to
  `=<version>`.
- Rename `## Unreleased` in `CHANGELOG.md` to `## <version> - <date>` and add
  a fresh empty `## Unreleased` above it. Each entry's first sentence says
  what the release is for.
- Point `smoke/mixed/enums/Cargo.toml` at the minor before the one being
  released, so the two-version graph `python3 smoke/mixed/check.py` builds
  stays one step wide.
- When the generated code or the benchmarks changed and a number is quoted,
  rerun the suite on an otherwise idle machine with
  `benchmarks/scripts/run.sh`, which regenerates `benchmarks/RESULTS.md`,
  and update the numbers the README and `docs/when-to-use-enum-dispatch.md`
  quote from it. Never substitute a shared-runner timing.
- Commit as `Release <version>`; the commit touches the manifests and the
  changelog.

## 2. Reproduce the local gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --test expand -- --ignored
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p whichever -p whichever-macros
cargo check -p whichever --no-default-features --tests
cargo check -p smoke-no-std --target thumbv7em-none-eabi
python3 smoke/mixed/check.py
cargo bench -p whichever-benchmarks --no-run
python3 benchmarks/scripts/codegen.py
python3 benchmarks/scripts/check_numbers.py
```

The compile-fail snapshots are pinned to stable's wording, so update the
local `stable` first (`rustup update stable`): a stale one passes a snapshot
that CI's current stable rejects. CI runs the other toolchains with
`-- --skip compile_fail`.

## 3. Inspect the packages

```sh
cargo package --list -p whichever-macros
cargo package --list -p whichever
```

The `whichever` archive carries the source, the README, the changelog, the
acknowledgements, this checklist and the licenses; `Cargo.toml` excludes the
smoke crates, the benchmarks, `docs/`, the workflows, the toolchain file,
the planning notes and `CLAUDE.md`.

## 4. Publish and tag

1. `cargo publish -p whichever-macros`, and wait until the new version
   resolves from the registry.
2. `cargo publish -p whichever`.
3. Tag the release commit `v<version>`, push the commit and the tag, and
   check that docs.rs builds both crates.
4. Leave the worktree clean. A post-release issue goes in the planning
   notes, not into a historical release entry.
