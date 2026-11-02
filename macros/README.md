# whichever-macros

The procedural macros behind [`whichever`](https://crates.io/crates/whichever),
which implements a trait for an enum by forwarding to whichever variant it
holds.

This crate is an implementation detail. Rust lets a proc-macro crate export
nothing but its macros, so `whichever` re-exports `#[dispatchable]` and
`#[dispatch]` from here and adds everything else around them: the
documentation, the `whichever::foreign` declarations of standard library
traits, and the hidden entry points the generated code calls back through.
Depend on `whichever` and you get this crate with it; nothing in it is meant
to be named directly.

- Documentation: <https://docs.rs/whichever>
- Repository and changelog: <https://github.com/shergin/whichever>

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
