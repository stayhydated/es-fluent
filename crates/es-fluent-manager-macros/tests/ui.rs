#[test]
fn bevy_fluent_text_failures_match_user_diagnostics() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
    tests.pass("tests/ui-pass/*.rs");
}
