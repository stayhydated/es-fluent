use super::inventory::collect_type_infos;
use super::*;
use es_fluent::registry::{FtlTypeInfo, FtlVariant, NamespaceRule};
use es_fluent_shared::meta::TypeKind;
use std::borrow::Cow;
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use tempfile::tempdir;

static EMPTY_VARIANTS: &[FtlVariant] = &[];
static ALLOWED_INFO: FtlTypeInfo = FtlTypeInfo {
    type_kind: TypeKind::Struct,
    type_name: "AllowedType",
    variants: EMPTY_VARIANTS,
    file_path: "src/lib.rs",
    module_path: "test_crate",
    namespace: Some(NamespaceRule::Literal(Cow::Borrowed("ui"))),
};
static DISALLOWED_INFO: FtlTypeInfo = FtlTypeInfo {
    type_kind: TypeKind::Struct,
    type_name: "DisallowedType",
    variants: EMPTY_VARIANTS,
    file_path: "src/lib.rs",
    module_path: "test_crate",
    namespace: Some(NamespaceRule::Literal(Cow::Borrowed("errors"))),
};
static CLEAN_VARIANTS: &[FtlVariant] = &[FtlVariant {
    name: "Key1",
    ftl_key: "group_a-Key1",
    args: &[],
    module_path: "test",
    line: 0,
}];
static CLEAN_INFO: FtlTypeInfo = FtlTypeInfo {
    type_kind: TypeKind::Enum,
    type_name: "GroupA",
    variants: CLEAN_VARIANTS,
    file_path: "src/lib.rs",
    module_path: "coverage_test_crate",
    namespace: None,
};
static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

es_fluent::__inventory::submit! {
    es_fluent::registry::RegisteredFtlType(&CLEAN_INFO)
}

fn with_env_var<T>(key: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().expect("lock poisoned");
    let previous = std::env::var_os(key);

    match value {
        Some(value) => {
            // SAFETY: tests serialize environment updates with a global lock.
            unsafe { std::env::set_var(key, value) };
        },
        None => {
            // SAFETY: tests serialize environment updates with a global lock.
            unsafe { std::env::remove_var(key) };
        },
    }

    let result = f();

    match previous {
        Some(previous) => {
            // SAFETY: tests serialize environment updates with a global lock.
            unsafe { std::env::set_var(key, previous) };
        },
        None => {
            // SAFETY: tests serialize environment updates with a global lock.
            unsafe { std::env::remove_var(key) };
        },
    }

    result
}

fn with_env_vars<T>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().expect("lock poisoned");
    let previous: Vec<(String, Option<std::ffi::OsString>)> = vars
        .iter()
        .map(|(key, _)| ((*key).to_string(), std::env::var_os(key)))
        .collect();

    for (key, value) in vars {
        match value {
            Some(value) => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::set_var(key, value) };
            },
            None => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::remove_var(key) };
            },
        }
    }

    let result = f();

    for (key, value) in previous {
        match value {
            Some(value) => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::set_var(&key, value) };
            },
            None => {
                // SAFETY: tests serialize environment updates with a global lock.
                unsafe { std::env::remove_var(&key) };
            },
        }
    }

    result
}

fn write_basic_i18n_config(manifest_dir: &Path) {
    std::fs::create_dir_all(manifest_dir.join("i18n/en-US")).expect("mkdir en-US");
    std::fs::create_dir_all(manifest_dir.join("i18n/fr")).expect("mkdir fr");
    std::fs::write(
        manifest_dir.join("i18n.toml"),
        "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\nnamespaces = [\"ui\"]\n",
    )
    .expect("write i18n.toml");
}

#[test]
fn resolve_helpers_use_overrides_and_config_defaults() {
    let temp = tempdir().expect("tempdir");
    write_basic_i18n_config(temp.path());

    let output_override = temp.path().join("custom-output");
    let assets_override = temp.path().join("custom-assets");
    let generator = EsFluentGenerator::builder()
        .crate_name("my-crate")
        .output_path(&output_override)
        .assets_dir(&assets_override)
        .manifest_dir(temp.path())
        .build();

    assert_eq!(
        generator.resolve_crate_name().expect("crate name"),
        "my-crate"
    );
    assert_eq!(
        generator.resolve_output_path().expect("output"),
        output_override
    );
    assert_eq!(
        generator.resolve_assets_dir().expect("assets"),
        assets_override
    );
    assert_eq!(
        generator.resolve_manifest_dir().expect("manifest"),
        temp.path()
    );
}

#[test]
fn resolve_helpers_can_load_defaults_from_manifest_environment() {
    let temp = tempdir().expect("tempdir");
    write_basic_i18n_config(temp.path());

    with_env_var("CARGO_MANIFEST_DIR", temp.path().to_str(), || {
        let generator = EsFluentGenerator::builder()
            .crate_name("missing-crate")
            .build();
        assert_eq!(
            generator.resolve_output_path().expect("output path"),
            temp.path().join("i18n/en-US")
        );
        assert_eq!(
            generator.resolve_assets_dir().expect("assets path"),
            temp.path().join("i18n")
        );
        assert_eq!(
            generator.resolve_manifest_dir().expect("manifest path"),
            temp.path()
        );
    });
}

#[test]
fn resolve_manifest_dir_reports_missing_environment() {
    let generator = EsFluentGenerator::builder()
        .crate_name("missing-crate")
        .build();

    with_env_var("CARGO_MANIFEST_DIR", None, || {
        let err = generator
            .resolve_manifest_dir()
            .expect_err("missing env should fail");
        assert!(
            matches!(err, GeneratorError::CrateName(message) if message.contains("CARGO_MANIFEST_DIR not set"))
        );
    });
}

#[test]
fn resolve_helpers_report_config_errors_when_manifest_lacks_i18n_toml() {
    let temp = tempdir().expect("tempdir");
    let generator = EsFluentGenerator::builder()
        .crate_name("missing-crate")
        .manifest_dir(temp.path())
        .build();

    let output_err = generator
        .resolve_output_path()
        .expect_err("missing config should fail");
    assert!(matches!(output_err, GeneratorError::Config(_)));

    let assets_err = generator
        .resolve_assets_dir()
        .expect_err("missing config should fail");
    assert!(matches!(assets_err, GeneratorError::Config(_)));
}

#[test]
fn resolve_clean_paths_supports_single_or_all_locales() {
    let temp = tempdir().expect("tempdir");
    write_basic_i18n_config(temp.path());

    let generator = EsFluentGenerator::builder()
        .crate_name("missing-crate")
        .manifest_dir(temp.path())
        .build();

    let single = generator
        .resolve_clean_paths(false)
        .expect("single clean path");
    assert_eq!(single, vec![temp.path().join("i18n/en-US")]);

    let all = generator
        .resolve_clean_paths(true)
        .expect("all clean paths");
    assert_eq!(
        all,
        vec![temp.path().join("i18n/en-US"), temp.path().join("i18n/fr")]
    );
}

#[test]
fn resolve_clean_paths_preserves_raw_locale_directory_names() {
    let temp = tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("i18n/en-us")).expect("mkdir en-us");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("mkdir fr");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en-us\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");

    let generator = EsFluentGenerator::builder()
        .crate_name("missing-crate")
        .manifest_dir(temp.path())
        .build();

    let all = generator
        .resolve_clean_paths(true)
        .expect("all clean paths");
    assert_eq!(
        all,
        vec![temp.path().join("i18n/en-us"), temp.path().join("i18n/fr")]
    );
}

#[test]
fn resolve_clean_paths_honors_assets_dir_override_for_all_locales() {
    let temp = tempdir().expect("tempdir");
    write_basic_i18n_config(temp.path());

    let override_assets = temp.path().join("custom-assets");
    std::fs::create_dir_all(override_assets.join("es-MX")).expect("mkdir es-MX");
    std::fs::create_dir_all(override_assets.join("ja")).expect("mkdir ja");

    let generator = EsFluentGenerator::builder()
        .crate_name("missing-crate")
        .manifest_dir(temp.path())
        .assets_dir(&override_assets)
        .build();

    let all = generator
        .resolve_clean_paths(true)
        .expect("all clean paths");
    assert_eq!(
        all,
        vec![override_assets.join("es-MX"), override_assets.join("ja")]
    );
}

#[test]
fn resolve_clean_paths_tolerates_invalid_locale_directory_names() {
    let temp = tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("i18n/en-US")).expect("mkdir en-US");
    std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("mkdir fr");
    std::fs::create_dir_all(temp.path().join("i18n/not_a_locale")).expect("mkdir invalid");
    std::fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");

    let generator = EsFluentGenerator::builder()
        .crate_name("missing-crate")
        .manifest_dir(temp.path())
        .build();

    let all = generator
        .resolve_clean_paths(true)
        .expect("all clean paths");
    assert_eq!(
        all,
        vec![
            temp.path().join("i18n/en-US"),
            temp.path().join("i18n/fr"),
            temp.path().join("i18n/not_a_locale"),
        ]
    );
}

#[test]
fn resolve_clean_paths_falls_back_to_output_override_when_assets_dir_missing() {
    let temp = tempdir().expect("tempdir");
    let fallback_output = temp.path().join("fallback-output");
    let generator = EsFluentGenerator::builder()
        .crate_name("missing-crate")
        .manifest_dir(temp.path())
        .output_path(&fallback_output)
        .assets_dir(temp.path().join("missing-assets"))
        .build();

    let paths = generator
        .resolve_clean_paths(true)
        .expect("resolve clean paths");
    assert_eq!(paths, vec![fallback_output]);
}

#[test]
fn validate_namespaces_allows_configured_namespaces_only() {
    let temp = tempdir().expect("tempdir");
    write_basic_i18n_config(temp.path());

    validate_namespaces(&[&ALLOWED_INFO], temp.path()).expect("allowed namespace should pass");

    let err = validate_namespaces(&[&DISALLOWED_INFO], temp.path())
        .expect_err("disallowed namespace should fail");
    assert!(matches!(
        err,
        GeneratorError::InvalidNamespace {
            namespace,
            type_name,
            ..
        } if namespace == "errors" && type_name == "DisallowedType"
    ));
}

#[test]
fn generate_and_clean_handle_empty_inventory() {
    let temp = tempdir().expect("tempdir");
    write_basic_i18n_config(temp.path());

    let generator = EsFluentGenerator::builder()
        .crate_name("missing-crate")
        .manifest_dir(temp.path())
        .build();

    let generate_changed = generator.generate().expect("generate");
    assert!(!generate_changed);

    let clean_changed = generator.clean(false, false).expect("clean");
    assert!(!clean_changed);

    let clean_all_changed = generator.clean(true, true).expect("clean all");
    assert!(!clean_all_changed);
}

#[test]
fn clean_marks_changes_when_cleaner_rewrites_files() {
    let temp = tempdir().expect("tempdir");
    write_basic_i18n_config(temp.path());

    let target_file = temp.path().join("i18n/en-US/coverage-test-crate.ftl");
    std::fs::write(
        &target_file,
        "## GroupA\n\ngroup_a-Key1 = Keep\norphan-Old = stale value\n",
    )
    .expect("write stale ftl");

    let generator = EsFluentGenerator::builder()
        .crate_name("coverage-test-crate")
        .manifest_dir(temp.path())
        .build();

    let changed = generator.clean(false, false).expect("clean");
    assert!(changed);
}

#[test]
fn detect_crate_name_works_in_test_environment() {
    with_env_vars(
        &[
            ("CARGO_MANIFEST_DIR", Some(env!("CARGO_MANIFEST_DIR"))),
            ("CARGO_PKG_NAME", Some(env!("CARGO_PKG_NAME"))),
        ],
        || {
            let crate_name = EsFluentGenerator::detect_crate_name().expect("crate name");
            assert_eq!(crate_name, env!("CARGO_PKG_NAME"));
        },
    );
}

#[test]
fn detect_crate_name_uses_env_fallback_or_errors_when_unavailable() {
    let temp = tempdir().expect("tempdir");

    with_env_vars(
        &[
            ("CARGO_MANIFEST_DIR", temp.path().to_str()),
            ("CARGO_PKG_NAME", Some("env-fallback-crate")),
        ],
        || {
            let crate_name = EsFluentGenerator::detect_crate_name().expect("crate name");
            assert_eq!(crate_name, "env-fallback-crate");
        },
    );

    with_env_vars(
        &[
            ("CARGO_MANIFEST_DIR", temp.path().to_str()),
            ("CARGO_PKG_NAME", None),
        ],
        || {
            let err = EsFluentGenerator::detect_crate_name().expect_err("should fail");
            assert!(
                matches!(err, GeneratorError::CrateName(message) if message.contains("Could not determine crate name"))
            );
        },
    );

    with_env_var("CARGO_MANIFEST_DIR", None, || {
        let err = EsFluentGenerator::detect_crate_name().expect_err("missing env should fail");
        assert!(
            matches!(err, GeneratorError::CrateName(message) if message.contains("CARGO_MANIFEST_DIR not set"))
        );
    });
}

#[test]
fn env_helpers_restore_unset_variables() {
    let key = format!("ES_FLUENT_TEST_UNSET_{}_A", std::process::id());
    with_env_var(&key, Some("value"), || {
        assert_eq!(std::env::var(&key).expect("set"), "value");
    });
    assert!(std::env::var(&key).is_err());

    let key_a = format!("ES_FLUENT_TEST_UNSET_{}_B", std::process::id());
    let key_b = format!("ES_FLUENT_TEST_UNSET_{}_C", std::process::id());
    with_env_vars(
        &[(key_a.as_str(), Some("first")), (key_b.as_str(), None)],
        || {
            assert_eq!(std::env::var(&key_a).expect("set"), "first");
            assert!(std::env::var(&key_b).is_err());
        },
    );
    assert!(std::env::var(&key_a).is_err());
    assert!(std::env::var(&key_b).is_err());
}

#[test]
fn collect_type_infos_returns_empty_for_unknown_crate() {
    let infos = collect_type_infos("definitely_unknown_crate_name");
    assert!(infos.is_empty());
}
