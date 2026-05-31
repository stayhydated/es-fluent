#[test]
fn macro_failures_match_user_diagnostics() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
