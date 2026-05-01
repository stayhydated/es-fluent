#[test]
fn define_i18n_module_rejects_arguments() {
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/ui/define_i18n_module_args.rs");
}
