#[serial_test::serial(manifest)]
#[test]
fn configured_missing_fallback_message_obeys_package_local_policy() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_dir = temp.path().join("fallback-app");
    let src_dir = crate_dir.join("src");
    let locale_dir = crate_dir.join("i18n/en");
    let target_dir = temp.path().join("target");
    let workspace_crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");

    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "fallback-app"
version = "0.1.0"
edition = "2024"

[dependencies]
es-fluent = {{ path = "{}" }}

[build-dependencies]
es-fluent-build = {{ path = "{}" }}
"#,
            toml_path(&workspace_crates.join("es-fluent")),
            toml_path(Path::new(env!("CARGO_MANIFEST_DIR"))),
        ),
    )
    .expect("write Cargo.toml");
    fs::write(crate_dir.join("build.rs"), BUILD_TRACK_I18N_SOURCE).expect("write build.rs");
    fs::write(
        src_dir.join("lib.rs"),
        "#[derive(es_fluent::EsFluent)]\npub struct MissingValue;\n",
    )
    .expect("write lib.rs");
    fs::write(
        crate_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    fs::write(locale_dir.join("fallback-app.ftl"), "present = Present\n")
        .expect("write fallback FTL");

    let strict = cargo_check_output(&crate_dir, &target_dir, &[]);
    assert!(
        !strict.status.success(),
        "strict build should reject the missing key"
    );
    let strict_stderr = String::from_utf8_lossy(&strict.stderr);
    for expected in [
        "missing fallback Fluent message `missing_value`",
        "domain `fallback-app`",
        "Rust item `MissingValue`",
        "pub struct MissingValue",
        "expected a message value under `i18n/en`",
        "cargo es-fluent generate --package fallback-app",
    ] {
        assert!(
            strict_stderr.contains(expected),
            "expected {expected:?} in strict stderr: {strict_stderr}"
        );
    }
    assert!(!strict_stderr.contains("E0080"));
    assert!(!strict_stderr.contains("OUT_DIR"));

    fs::write(
        locale_dir.join("fallback-app.ftl"),
        "missing_value = Missing value\n",
    )
    .expect("write complete fallback FTL");
    let complete = cargo_check_output(&crate_dir, &target_dir, &[]);
    assert!(
        complete.status.success(),
        "strict build should accept the fallback key: {}",
        String::from_utf8_lossy(&complete.stderr)
    );

    fs::write(locale_dir.join("fallback-app.ftl"), "present = Present\n")
        .expect("restore missing fallback FTL");
    fs::write(
            crate_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\nmissing_message_policy = \"fallback-str\"\n",
        )
        .expect("write fallback policy");
    let fallback = cargo_check_output(&crate_dir, &target_dir, &[]);
    assert!(
        fallback.status.success(),
        "fallback-str build should succeed: {}",
        String::from_utf8_lossy(&fallback.stderr)
    );
}

#[serial_test::serial(manifest)]
#[test]
fn configured_strict_crate_allows_doctest_only_derives() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_dir = temp.path().join("strict-doctest");
    let src_dir = crate_dir.join("src");
    let locale_dir = crate_dir.join("i18n/en");
    let target_dir = temp.path().join("target");
    let workspace_crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"strict-doctest\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent = {{ path = \"{}\" }}\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{}\" }}\n",
                toml_path(&workspace_crates.join("es-fluent")),
                toml_path(Path::new(env!("CARGO_MANIFEST_DIR"))),
            ),
        )
        .expect("write Cargo.toml");
    fs::write(crate_dir.join("build.rs"), BUILD_TRACK_I18N_SOURCE).expect("write build.rs");
    fs::write(
        src_dir.join("lib.rs"),
        r#"/// ```
/// #[derive(es_fluent::EsFluent)]
/// struct DoctestOnly;
/// ```
pub struct Documentation;

#[derive(es_fluent::EsFluent)]
pub struct LibraryValue;
"#,
    )
    .expect("write lib.rs");
    fs::write(
        crate_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    fs::write(
        locale_dir.join("strict-doctest.ftl"),
        "library_value = Library value\n",
    )
    .expect("write fallback FTL");

    let output = cargo_workspace_output(&crate_dir, &target_dir, &["test", "--quiet", "--doc"]);
    assert!(
        output.status.success(),
        "doctest-only derives should not require library fallback inventory: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[serial_test::serial(manifest)]
#[test]
fn configured_strict_crate_allows_test_only_derives() {
    let temp = tempfile::tempdir().expect("tempdir");
    let crate_dir = temp.path().join("strict-tests");
    let src_dir = crate_dir.join("source");
    let locale_dir = crate_dir.join("i18n/en");
    let target_dir = temp.path().join("target");
    let workspace_crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::create_dir_all(crate_dir.join("tests")).expect("create integration test dir");
    fs::create_dir_all(crate_dir.join("qa")).expect("create custom test dir");
    fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"strict-tests\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"source/library.rs\"\n\n[dependencies]\nes-fluent = {{ path = \"{}\" }}\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{}\" }}\n\n[[test]]\nname = \"custom-integration\"\npath = \"qa/custom.rs\"\n",
                toml_path(&workspace_crates.join("es-fluent")),
                toml_path(Path::new(env!("CARGO_MANIFEST_DIR"))),
            ),
        )
        .expect("write Cargo.toml");
    fs::write(crate_dir.join("build.rs"), BUILD_TRACK_I18N_SOURCE).expect("write build.rs");
    fs::write(
        src_dir.join("library.rs"),
        r#"#[derive(es_fluent::EsFluent)]
pub struct LibraryValue;

macro_rules! define_message {
    ($name:ident) => {
        #[derive(es_fluent::EsFluent)]
        pub struct $name;
    };
}

define_message!(MacroGenerated);

include!("included_production.rs");

pub fn production_block_include() {
    include!("included_production_block.rs");
}

pub fn production_expression_include() {
    let _message = include!("included_production_expression.rs");
}

include!("included_nested_production/items.rs");

#[cfg(test)]
mod macro_name_decoy {
    pub struct MacroGenerated;
}

#[cfg(test)]
mod macro_generated_tests {
    define_message!(MacroGeneratedTestOnly);

    #[test]
    fn generated_test_only_message_is_usable() {
        let _message = MacroGeneratedTestOnly;
    }
}

#[cfg(test)]
mod included_tests {
    include!("included_tests.rs");
}

#[cfg(test)]
mod included_nested_tests {
    include!("included_nested_tests/items.rs");
}

#[cfg(test)]
#[derive(es_fluent::EsFluent)]
struct TestOnlyMessage;

#[cfg(test)]
#[derive(es_fluent::EsFluentLabel)]
struct TestOnlyLabel;

#[cfg(test)]
mod nested_tests {
    #[derive(es_fluent::EsFluent)]
    struct NestedTestOnly;

    #[test]
    fn nested_test_only_derive_is_usable() {
        let _message = NestedTestOnly;
    }
}

#[cfg(test)]
mod out_of_line_tests;

#[path = "../tests/production_shared.rs"]
mod production_shared;

mod production_names {
    pub struct ReusedName;
}

#[cfg(test)]
mod duplicate_name_tests {
    #[derive(es_fluent::EsFluent)]
    struct ReusedName;

    #[test]
    fn duplicate_test_name_is_usable() {
        let _message = ReusedName;
    }
}

#[cfg_attr(test, derive(es_fluent::EsFluent))]
struct CfgAttrMessageOnly;

#[derive(es_fluent::EsFluent)]
#[cfg_attr(test, derive(es_fluent::EsFluentLabel))]
struct MixedDerivePolicy;

#[cfg_attr(test, derive(es_fluent::EsFluentVariants))]
struct CfgAttrVariantsOnly {
    field: String,
}

#[cfg_attr(test, derive(es_fluent::EsFluentChoice))]
enum CfgAttrChoiceOnly {
    Value,
}

#[cfg(test)]
struct TestLocalizer;

#[cfg(test)]
impl es_fluent::FluentLocalizer for TestLocalizer {
    fn localize<'a>(
        &self,
        _key: es_fluent::registry::StaticFluentMessageKey,
        _args: Option<&'a es_fluent::FluentArgs<'a>>,
    ) -> Option<String> {
        Some("localized".to_string())
    }
}

#[cfg(test)]
#[test]
fn test_only_derives_are_usable() {
    use es_fluent::{FluentLabel as _, FluentLocalizerExt as _};

    assert_eq!(
        TestLocalizer.localize_message(&TestOnlyMessage),
        "localized"
    );
    assert_eq!(TestOnlyLabel::localize_label(&TestLocalizer), "localized");
    assert_eq!(
        MixedDerivePolicy::localize_label(&TestLocalizer),
        "localized"
    );
    let _message = CfgAttrMessageOnly;
    let _variants = CfgAttrVariantsOnly {
        field: String::new(),
    };
    let _choice = CfgAttrChoiceOnly::Value;
}

#[test]
fn block_local_test_derive_is_usable() {
    if true {
        #[derive(es_fluent::EsFluent)]
        struct BlockLocalTestOnly;

        let _message = BlockLocalTestOnly;
    }
}

#[test]
fn block_local_included_test_derive_is_usable() {
    include!("included_block_test.rs");
}

#[test]
fn expression_included_test_derive_is_usable() {
    let _message = include!("included_expression_test.rs");
}
"#,
    )
    .expect("write lib.rs");
    fs::write(
        src_dir.join("out_of_line_tests.rs"),
        r#"#[derive(es_fluent::EsFluent)]
struct OutOfLineOnly;

#[test]
fn out_of_line_derive_is_usable() {
    let _message = OutOfLineOnly;
}
"#,
    )
    .expect("write out-of-line test module");
    fs::write(
        src_dir.join("included_tests.rs"),
        r#"#[derive(es_fluent::EsFluent)]
struct IncludedTestOnly;

#[test]
fn included_test_only_derive_is_usable() {
    let _message = IncludedTestOnly;
}
"#,
    )
    .expect("write included test source");
    fs::write(
        src_dir.join("included_production.rs"),
        "#[derive(es_fluent::EsFluent)]\npub struct IncludedProduction;\n",
    )
    .expect("write included production source");
    fs::write(
        src_dir.join("included_block_test.rs"),
        r#"{
#[derive(es_fluent::EsFluent)]
struct IncludedBlockTestOnly;

let _message = IncludedBlockTestOnly;
}
"#,
    )
    .expect("write block-included test source");
    fs::write(
        src_dir.join("included_production_block.rs"),
        r#"{
#[derive(es_fluent::EsFluent)]
struct IncludedProductionBlock;

let _message = IncludedProductionBlock;
}
"#,
    )
    .expect("write block-included production source");
    fs::write(
        src_dir.join("included_production_expression.rs"),
        r#"{
#[derive(es_fluent::EsFluent)]
struct IncludedProductionExpression;

IncludedProductionExpression
}
"#,
    )
    .expect("write expression-included production source");
    fs::write(
        src_dir.join("included_expression_test.rs"),
        r#"{
#[derive(es_fluent::EsFluent)]
struct IncludedExpressionTestOnly;

IncludedExpressionTestOnly
}
"#,
    )
    .expect("write expression-included test source");
    fs::create_dir_all(src_dir.join("included_nested_tests"))
        .expect("create nested included test directory");
    fs::write(
        src_dir.join("included_nested_tests/items.rs"),
        "mod messages;\n",
    )
    .expect("write nested included test items");
    fs::write(
        src_dir.join("included_nested_tests/messages.rs"),
        "#[derive(es_fluent::EsFluent)]\nstruct IncludedNestedTestOnly;\n",
    )
    .expect("write nested included test module");
    fs::create_dir_all(src_dir.join("included_nested_production"))
        .expect("create nested included production directory");
    fs::write(
        src_dir.join("included_nested_production/items.rs"),
        "mod messages;\n",
    )
    .expect("write nested included production items");
    fs::write(
        src_dir.join("included_nested_production/messages.rs"),
        "#[derive(es_fluent::EsFluent)]\npub struct IncludedNestedProduction;\n",
    )
    .expect("write nested included production module");
    fs::write(
        crate_dir.join("tests/integration.rs"),
        r#"macro_rules! define_message {
    ($name:ident) => {
        #[derive(es_fluent::EsFluent)]
        struct $name;
    };
}

define_message!(IntegrationMacroOnly);

#[derive(es_fluent::EsFluent)]
struct IntegrationOnly;

#[test]
fn integration_derive_is_usable() {
    let _message = IntegrationOnly;
    let _generated = IntegrationMacroOnly;
}
"#,
    )
    .expect("write integration test target");
    fs::write(
        crate_dir.join("tests/production_shared.rs"),
        "#[derive(es_fluent::EsFluent)]\npub struct ProductionShared;\n",
    )
    .expect("write production module under tests directory");
    fs::write(
        crate_dir.join("qa/custom.rs"),
        r#"#[derive(es_fluent::EsFluent)]
struct CustomIntegrationOnly;

#[test]
fn custom_integration_derive_is_usable() {
    let _message = CustomIntegrationOnly;
}
"#,
    )
    .expect("write custom integration test target");
    fs::write(
        crate_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    fs::write(
            locale_dir.join("strict-tests.ftl"),
            "library_value = Library value\nmacro_generated = Macro generated\nincluded_production = Included production\nincluded_production_block = Included production block\nincluded_production_expression = Included production expression\nincluded_nested_production = Included nested production\nmixed_derive_policy = Mixed derive policy\nproduction_shared = Production shared\n",
        )
        .expect("write fallback FTL");

    let output = cargo_workspace_output(&crate_dir, &target_dir, &["test", "--quiet", "--lib"]);
    assert!(
        output.status.success(),
        "test-only derives should not require generated fallback inventory: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for target in ["integration", "custom-integration"] {
        let output = cargo_workspace_output(
            &crate_dir,
            &target_dir,
            &["test", "--quiet", "--test", target],
        );
        assert!(
            output.status.success(),
            "{target} derives should be coverage-exempt: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::write(
            locale_dir.join("strict-tests.ftl"),
            "macro_generated = Macro generated\nincluded_production = Included production\nincluded_production_block = Included production block\nincluded_production_expression = Included production expression\nincluded_nested_production = Included nested production\nmixed_derive_policy = Mixed derive policy\nproduction_shared = Production shared\n",
        )
        .expect("remove library fallback value");
    let missing_library =
        cargo_workspace_output(&crate_dir, &target_dir, &["test", "--quiet", "--lib"]);
    assert!(
        !missing_library.status.success(),
        "normal library derives must remain strict in test builds"
    );
    let stderr = String::from_utf8_lossy(&missing_library.stderr);
    assert!(
        stderr.contains("missing fallback Fluent message `library_value`"),
        "strict library diagnostic should remain active: {stderr}"
    );
    assert!(
        !stderr.contains("missing fallback Fluent message `test_only_message`")
            && !stderr.contains("missing fallback Fluent message `test_only_label_label`")
            && !stderr.contains("missing fallback Fluent message `nested_test_only`")
            && !stderr.contains("missing fallback Fluent message `out_of_line_only`")
            && !stderr.contains("missing fallback Fluent message `reused_name`")
            && !stderr.contains("missing fallback Fluent message `cfg_attr_message_only`")
            && !stderr.contains("missing fallback Fluent message `mixed_derive_policy_label`")
            && !stderr.contains("cfg_attr_variants_only_variants")
            && !stderr.contains("macro_generated_test_only")
            && !stderr.contains("included_test_only")
            && !stderr.contains("included_block_test_only")
            && !stderr.contains("included_expression_test_only")
            && !stderr.contains("included_nested_test_only")
            && !stderr.contains("integration_macro_only")
            && !stderr.contains("block_local_test_only"),
        "test-only derives should remain coverage-exempt: {stderr}"
    );

    fs::write(
        locale_dir.join("strict-tests.ftl"),
        "library_value = Library value\nmixed_derive_policy = Mixed derive policy\n",
    )
    .expect("remove macro-generated fallback value");
    let macro_generated =
        cargo_workspace_output(&crate_dir, &target_dir, &["check", "--quiet", "--lib"]);
    assert!(
        !macro_generated.status.success(),
        "macro-generated production derives must remain strict"
    );
    let stderr = String::from_utf8_lossy(&macro_generated.stderr);
    assert!(
        stderr.contains("missing fallback Fluent message `macro_generated`"),
        "test-only same-name source must not exempt macro output: {stderr}"
    );
    assert!(
        stderr.contains("missing fallback Fluent message `production_shared`"),
        "a production module under tests/ must remain strict: {stderr}"
    );
    assert!(
        stderr.contains("missing fallback Fluent message `included_production`"),
        "a production literal include must remain strict: {stderr}"
    );
    assert!(
        stderr.contains("missing fallback Fluent message `included_production_block`"),
        "a production block-local literal include must remain strict: {stderr}"
    );
    assert!(
        stderr.contains("missing fallback Fluent message `included_production_expression`"),
        "a production expression literal include must remain strict: {stderr}"
    );
    assert!(
        stderr.contains("missing fallback Fluent message `included_nested_production`"),
        "an out-of-line module below a production literal include must remain strict: {stderr}"
    );
}

#[serial_test::serial(manifest)]
#[test]
fn mixed_workspace_keeps_missing_message_policy_package_local() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_dir = temp.path().join("mixed-policy");
    let strict_dir = workspace_dir.join("strict-app");
    let fallback_dir = workspace_dir.join("fallback-app");
    let target_dir = temp.path().join("target");
    let workspace_crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory");
    fs::create_dir_all(strict_dir.join("src")).expect("create strict src");
    fs::create_dir_all(strict_dir.join("i18n/en")).expect("create strict locale");
    fs::create_dir_all(fallback_dir.join("src")).expect("create fallback src");
    fs::create_dir_all(fallback_dir.join("i18n/en")).expect("create fallback locale");
    fs::write(
        workspace_dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"strict-app\", \"fallback-app\"]\nresolver = \"3\"\n",
    )
    .expect("write workspace manifest");
    for (package, directory) in [("strict-app", &strict_dir), ("fallback-app", &fallback_dir)] {
        fs::write(
                directory.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nes-fluent = {{ path = \"{}\" }}\n\n[build-dependencies]\nes-fluent-build = {{ path = \"{}\" }}\n",
                    toml_path(&workspace_crates.join("es-fluent")),
                    toml_path(Path::new(env!("CARGO_MANIFEST_DIR"))),
                ),
            )
            .expect("write package manifest");
        fs::write(directory.join("build.rs"), BUILD_TRACK_I18N_SOURCE).expect("write build script");
    }
    fs::write(
        strict_dir.join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write strict config");
    fs::write(
            fallback_dir.join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\nmissing_message_policy = \"fallback-str\"\n",
        )
        .expect("write fallback config");
    fs::write(
        strict_dir.join("src/lib.rs"),
        "#[derive(es_fluent::EsFluent)]\npub struct MissingStrict;\n",
    )
    .expect("write strict source");
    fs::write(
            fallback_dir.join("src/lib.rs"),
            r#"#[derive(es_fluent::EsFluent)]
pub struct MissingFallback;

#[cfg(test)]
mod tests {
    use super::MissingFallback;
    use es_fluent::{FluentArgs, FluentLocalizer, FluentLocalizerExt as _};
    use es_fluent::registry::StaticFluentMessageKey;

    struct Missing;

    impl FluentLocalizer for Missing {
        fn localize<'a>(&self, _key: StaticFluentMessageKey, _args: Option<&FluentArgs<'a>>) -> Option<String> {
            None
        }
    }

    #[test]
    fn normal_and_fallible_lookup_keep_distinct_semantics() {
        assert_eq!(Missing.localize_message(&MissingFallback), "missing_fallback");
        assert_eq!(Missing.try_localize_message(&MissingFallback), None);
    }
}
"#,
        )
        .expect("write fallback source");
    fs::write(
        strict_dir.join("i18n/en/strict-app.ftl"),
        "present = Present\n",
    )
    .expect("write incomplete strict resource");
    fs::write(
        fallback_dir.join("i18n/en/fallback-app.ftl"),
        "present = Present\n",
    )
    .expect("write fallback resource");

    let strict = cargo_workspace_output(
        &workspace_dir,
        &target_dir,
        &["check", "--quiet", "-p", "strict-app"],
    );
    assert!(!strict.status.success());
    assert!(
        String::from_utf8_lossy(&strict.stderr)
            .contains("missing fallback Fluent message `missing_strict`")
    );

    let workspace = cargo_workspace_output(
        &workspace_dir,
        &target_dir,
        &["check", "--quiet", "--workspace"],
    );
    assert!(!workspace.status.success());
    let stderr = String::from_utf8_lossy(&workspace.stderr);
    assert!(stderr.contains("domain `strict-app`"), "{stderr}");
    assert!(stderr.contains("Rust item `MissingStrict`"), "{stderr}");

    fs::write(
        strict_dir.join("i18n/en/strict-app.ftl"),
        "missing_strict = Missing strict\n",
    )
    .expect("complete strict resource");
    let complete = cargo_workspace_output(
        &workspace_dir,
        &target_dir,
        &["test", "--quiet", "--workspace"],
    );
    assert!(
        complete.status.success(),
        "mixed workspace should pass after completing strict resources: {}",
        String::from_utf8_lossy(&complete.stderr)
    );
}
