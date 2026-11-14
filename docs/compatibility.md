# Compatibility

`no_std`, with no dependencies beyond the proc-macro crate. The minimum
supported Rust version is 1.88. Both the crate that owns the trait and the
crate that owns the enum depend on `whichever`; renaming the dependency is
fine under the default feature `renamed-dependency`, which looks the name
up through `proc-macro-crate`, and without it the macro crate's dependency
tree is `proc-macro2`, `quote` and `syn` alone. The `syn` is `syn` 3, the
one `serde_derive` and `thiserror` build, so a tree that has moved to it
gains no second `syn`. A crate that
`proc-macro-crate` itself depends on, such as toml_edit, takes `whichever`
with `default-features = false`, since with the feature on it would depend
on itself through `proc-macro-crate`, which Cargo rejects.

`#[dispatchable]` on a `pub` trait adds one `#[doc(hidden)]` exported macro,
the carrier macro, to the crate root, named after the trait and the position
of its tokens, which is how another crate's enum reaches the trait's
signatures. The carrier is read by whatever `whichever` the enum's crate
builds with, so before 1.0 the two crates use the same minor: a newer
`whichever` reads every older carrier, and an older one reports a newer
carrier in one sentence, since to Cargo two minors of a 0.x crate are two
crates and a graph may hold both. The macro holds the signatures and the attributes the generated
items carry, and nothing else: no default body, no doc comment, no bound. A trait
of any other visibility keeps its macro in its own module and adds nothing
to the crate root. A trait with items under `#[cfg]` defines the macro once
per combination of its predicates, each under that combination's `cfg`, so
that one definition of the name is compiled. The name is a function of the
trait's tokens and their positions, so an expansion is the same on every
build and on every machine; `#[dispatchable(name = ..)]` replaces the hash
when one `macro_rules!` emits the same trait twice.
