fn main() {
    let i18n_path = std::env::var("ES_FLUENT_TEST_I18N")
        .expect("ES_FLUENT_TEST_I18N must be set for this test binary");
    let crate_name = std::env::var("ES_FLUENT_TEST_CRATE")
        .expect("ES_FLUENT_TEST_CRATE must be set for this test binary");

    if let Err(error) = es_fluent_cli_helpers::run_generate(&i18n_path, &crate_name) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
