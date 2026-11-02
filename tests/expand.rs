//! Expansion snapshots: the `.expanded.rs` files next to the cases are what
//! the attributes generate, kept readable so the "plain match" claim can be
//! checked by eye.
#[test]
#[ignore = "needs cargo-expand; run with `cargo test --test expand -- --ignored`"]
fn expand() {
    macrotest::expand("tests/expand/*.rs");
}
