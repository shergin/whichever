//! Every unsupported shape produces one sentence on the offending item; the
//! `.stderr` files next to the cases pin those messages.
#[test]
fn compile_fail() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
