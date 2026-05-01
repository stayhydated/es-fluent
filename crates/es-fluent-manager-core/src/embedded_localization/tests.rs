use super::*;
use rust_embed::RustEmbed;
use std::borrow::Cow;
use unic_langid::langid;

#[derive(RustEmbed)]
#[folder = "tests/fixtures/embedded_i18n"]
struct TestAssets;

impl EmbeddedAssets for TestAssets {
    fn domain() -> &'static str {
        "test-domain"
    }

    fn namespaces() -> &'static [&'static str] {
        &["ui"]
    }
}

#[derive(RustEmbed)]
#[folder = "tests/fixtures/embedded_i18n"]
struct BaseFileAssets;

impl EmbeddedAssets for BaseFileAssets {
    fn domain() -> &'static str {
        "test-domain"
    }
}

#[derive(RustEmbed)]
#[folder = "tests/fixtures/embedded_i18n_ns_errors"]
struct NamespaceErrorAssets;

impl EmbeddedAssets for NamespaceErrorAssets {
    fn domain() -> &'static str {
        "test-domain"
    }

    fn namespaces() -> &'static [&'static str] {
        &["ui"]
    }
}

#[derive(RustEmbed)]
#[folder = "tests/fixtures/embedded_i18n_stray_base_file"]
struct StrayBaseFileAssets;

impl EmbeddedAssets for StrayBaseFileAssets {
    fn domain() -> &'static str {
        "test-domain"
    }

    fn namespaces() -> &'static [&'static str] {
        &["ui"]
    }
}

#[derive(RustEmbed)]
#[folder = "tests/fixtures/embedded_i18n_nested"]
struct NestedNamespaceAssets;

impl EmbeddedAssets for NestedNamespaceAssets {
    fn domain() -> &'static str {
        "test-domain"
    }

    fn namespaces() -> &'static [&'static str] {
        &["ui/button"]
    }
}

#[derive(RustEmbed)]
#[folder = "tests/fixtures/embedded_i18n_bundle_add_error"]
struct BundleAddErrorAssets;

impl EmbeddedAssets for BundleAddErrorAssets {
    fn domain() -> &'static str {
        "test-domain"
    }

    fn namespaces() -> &'static [&'static str] {
        &["ui", "errors"]
    }
}

#[derive(RustEmbed)]
#[folder = "tests/fixtures/embedded_i18n_partial_fallback"]
struct PartialFallbackAssets;

impl EmbeddedAssets for PartialFallbackAssets {
    fn domain() -> &'static str {
        "test-domain"
    }

    fn namespaces() -> &'static [&'static str] {
        &["ui"]
    }
}

struct OptionalOnlyAssets;

impl RustEmbed for OptionalOnlyAssets {
    fn get(_file_path: &str) -> Option<rust_embed::EmbeddedFile> {
        None
    }

    fn iter() -> impl Iterator<Item = Cow<'static, str>> + 'static {
        std::iter::empty()
    }
}

impl EmbeddedAssets for OptionalOnlyAssets {
    fn domain() -> &'static str {
        "test-domain"
    }

    fn resource_plan_for_language(_lang: &LanguageIdentifier) -> Option<Vec<ModuleResourceSpec>> {
        Some(vec![ModuleResourceSpec {
            key: ResourceKey::new("test-domain"),
            locale_relative_path: "test-domain.ftl".to_string(),
            required: false,
        }])
    }
}

#[test]
fn embedded_asset_test_types_expose_expected_domains_and_namespaces() {
    assert_eq!(TestAssets::domain(), "test-domain");
    assert_eq!(TestAssets::namespaces(), &["ui"]);
    assert_eq!(BaseFileAssets::domain(), "test-domain");
    assert!(BaseFileAssets::namespaces().is_empty());
    assert_eq!(NamespaceErrorAssets::domain(), "test-domain");
    assert_eq!(NamespaceErrorAssets::namespaces(), &["ui"]);
    assert_eq!(StrayBaseFileAssets::domain(), "test-domain");
    assert_eq!(StrayBaseFileAssets::namespaces(), &["ui"]);
    assert_eq!(NestedNamespaceAssets::domain(), "test-domain");
    assert_eq!(NestedNamespaceAssets::namespaces(), &["ui/button"]);
    assert_eq!(BundleAddErrorAssets::domain(), "test-domain");
    assert_eq!(BundleAddErrorAssets::namespaces(), &["ui", "errors"]);
    assert_eq!(PartialFallbackAssets::domain(), "test-domain");
    assert_eq!(PartialFallbackAssets::namespaces(), &["ui"]);
}

static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[
    langid!("en"),
    langid!("en-GB"),
    langid!("fr"),
    langid!("it"),
];
static NAMESPACES: &[&str] = &["ui"];
static MODULE_DATA: ModuleData = ModuleData {
    name: "test-module",
    domain: "test-domain",
    supported_languages: SUPPORTED_LANGUAGES,
    namespaces: NAMESPACES,
};
static BASE_FILE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static BASE_FILE_MODULE_DATA: ModuleData = ModuleData {
    name: "base-file-module",
    domain: "test-domain",
    supported_languages: BASE_FILE_SUPPORTED_LANGUAGES,
    namespaces: &[],
};
static NS_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("ab"), langid!("ef")];
static NS_ERROR_MODULE_DATA: ModuleData = ModuleData {
    name: "ns-error-module",
    domain: "test-domain",
    supported_languages: NS_ERROR_SUPPORTED_LANGUAGES,
    namespaces: NAMESPACES,
};
static STRAY_BASE_FILE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static STRAY_BASE_FILE_MODULE_DATA: ModuleData = ModuleData {
    name: "stray-base-file-module",
    domain: "test-domain",
    supported_languages: STRAY_BASE_FILE_SUPPORTED_LANGUAGES,
    namespaces: NAMESPACES,
};
static NESTED_NAMESPACE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static NESTED_NAMESPACE_MODULE_DATA: ModuleData = ModuleData {
    name: "nested-namespace-module",
    domain: "test-domain",
    supported_languages: NESTED_NAMESPACE_SUPPORTED_LANGUAGES,
    namespaces: &["ui/button"],
};
static BUNDLE_ADD_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
    &[langid!("en"), langid!("fr")];
static BUNDLE_ADD_ERROR_MODULE_DATA: ModuleData = ModuleData {
    name: "bundle-add-error-module",
    domain: "test-domain",
    supported_languages: BUNDLE_ADD_ERROR_SUPPORTED_LANGUAGES,
    namespaces: &["ui", "errors"],
};
static PARTIAL_FALLBACK_SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
    &[langid!("en-US"), langid!("en")];
static PARTIAL_FALLBACK_MODULE_DATA: ModuleData = ModuleData {
    name: "partial-fallback-module",
    domain: "test-domain",
    supported_languages: PARTIAL_FALLBACK_SUPPORTED_LANGUAGES,
    namespaces: NAMESPACES,
};
static OPTIONAL_ONLY_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static OPTIONAL_ONLY_MODULE_DATA: ModuleData = ModuleData {
    name: "optional-only-module",
    domain: "test-domain",
    supported_languages: OPTIONAL_ONLY_SUPPORTED_LANGUAGES,
    namespaces: &[],
};

#[test]
fn discover_languages_collects_and_sorts_unique_languages() {
    let languages = EmbeddedI18nModule::<TestAssets>::discover_languages();
    assert_eq!(
        languages,
        vec![langid!("en"), langid!("en-GB"), langid!("fr")]
    );
}

#[test]
fn discover_languages_supports_base_files_when_no_namespaces_are_configured() {
    assert!(BaseFileAssets::namespaces().is_empty());

    let languages = EmbeddedI18nModule::<BaseFileAssets>::discover_languages();
    assert_eq!(
        languages,
        vec![langid!("en"), langid!("en-GB"), langid!("fr")]
    );
}

#[test]
fn discover_languages_includes_locales_with_only_nested_namespace_files() {
    let languages = EmbeddedI18nModule::<NestedNamespaceAssets>::discover_languages();
    assert_eq!(languages, vec![langid!("en")]);
}

#[test]
fn embedded_language_discovery_only_accepts_canonical_resources() {
    assert_eq!(
        embedded_resource_from_asset_path("en/test-domain.ftl", "test-domain", &["ui"]),
        Some((langid!("en"), None))
    );
    assert_eq!(
        embedded_resource_from_asset_path("en/test-domain/ui.ftl", "test-domain", &["ui"]),
        Some((langid!("en"), Some("ui".to_string())))
    );
    assert_eq!(
        embedded_resource_from_asset_path(
            "en/test-domain/ui/button.ftl",
            "test-domain",
            &["ui/button"]
        ),
        Some((langid!("en"), Some("ui/button".to_string())))
    );
    assert_eq!(
        embedded_resource_from_asset_path("en/test-domain/readme.txt", "test-domain", &["ui"]),
        None
    );
    assert_eq!(
        embedded_resource_from_asset_path("en/test-domain/misc.ftl", "test-domain", &["ui"]),
        None
    );
    assert_eq!(
        embedded_resource_from_asset_path("iw/test-domain/ui.ftl", "test-domain", &["ui"]),
        None
    );
    assert_eq!(
        embedded_resource_from_asset_path("", "test-domain", &["ui"]),
        None
    );
    assert_eq!(
        embedded_resource_from_asset_path("en", "test-domain", &["ui"]),
        None
    );
    assert_eq!(
        embedded_resource_from_asset_path("en/other-domain/ui.ftl", "test-domain", &["ui"]),
        None
    );
    assert_eq!(
        embedded_resource_from_asset_path("en/test-domain.ftl/extra", "test-domain", &[]),
        None
    );
    assert_eq!(
        embedded_resource_from_asset_path("en/test-domain/.ftl", "test-domain", &["ui"]),
        None
    );
    assert_eq!(
        embedded_resource_from_asset_path("en-us/test-domain/ui.ftl", "test-domain", &["ui"]),
        None
    );
    assert_eq!(
        parse_embedded_language_identifier("en-US"),
        Some(langid!("en-US"))
    );
    assert_eq!(parse_embedded_language_identifier("en-us"), None);
}

#[test]
fn embedded_localizer_loads_base_file_modules_without_namespaces() {
    let localizer = EmbeddedLocalizer::<BaseFileAssets>::new(&BASE_FILE_MODULE_DATA);

    localizer
        .select_language(&langid!("en"))
        .expect("base file should make the locale ready");

    assert_eq!(
        localizer.localize("base-only", None),
        Some("Hello main".to_string())
    );
}

#[test]
fn embedded_localizer_uses_fallback_and_formats_with_args() {
    let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

    assert_eq!(localizer.localize("hello", None), None);

    localizer
        .select_language(&langid!("en-US"))
        .expect("fallback to en should work");

    let mut args = HashMap::new();
    args.insert("name", FluentValue::from("Mark"));
    let welcome = localizer.localize("welcome", Some(&args));
    assert!(
        welcome
            .as_deref()
            .is_some_and(|value| value.contains("Welcome"))
    );
    assert!(
        welcome
            .as_deref()
            .is_some_and(|value| value.contains("Mark"))
    );
    assert_eq!(
        localizer.localize("base-only", None),
        Some("Hello main".to_string())
    );
    assert_eq!(
        localizer.localize("ui-title", None),
        Some("UI Title".to_string())
    );

    localizer
        .select_language(&langid!("en"))
        .expect("re-selecting current language should no-op");
    localizer
        .select_language(&langid!("en"))
        .expect("re-selecting exactly the active language should no-op");
}

#[test]
fn embedded_localizer_preserves_requested_locale_in_bundle_metadata() {
    let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

    localizer
        .select_language(&langid!("en-US"))
        .expect("fallback to en should work");

    assert_eq!(
        localizer.current_lang.read().as_ref().cloned(),
        Some(langid!("en-US"))
    );

    let bundle = localizer
        .current_bundle
        .read()
        .as_ref()
        .cloned()
        .expect("bundle should be built");
    assert_eq!(bundle.locales, vec![langid!("en-US"), langid!("en")]);
}

#[test]
fn embedded_localizer_uses_fluent_fallback_for_missing_messages() {
    let localizer = EmbeddedLocalizer::<PartialFallbackAssets>::new(&PARTIAL_FALLBACK_MODULE_DATA);

    localizer
        .select_language(&langid!("en-US"))
        .expect("partial locale should fall back to en for missing messages");

    assert_eq!(
        localizer.localize("hello", None),
        Some("Hello from en-US".to_string())
    );
    assert_eq!(
        localizer.localize("ui-title", None),
        Some("Shared UI Title".to_string())
    );
}

#[test]
fn embedded_localizer_treats_missing_optional_only_resources_as_ready() {
    let localizer = EmbeddedLocalizer::<OptionalOnlyAssets>::new(&OPTIONAL_ONLY_MODULE_DATA);

    localizer
        .select_language(&langid!("en"))
        .expect("missing optional-only resources should still make the locale ready");

    assert_eq!(
        localizer.current_lang.read().as_ref().cloned(),
        Some(langid!("en"))
    );
    assert_eq!(localizer.localize("missing-message", None), None);
}

#[test]
fn embedded_localizer_exercises_fallback_and_missing_resource_paths() {
    let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

    // en-GB does not have a ready canonical namespace resource, so
    // selection should fall back to en.
    localizer
        .select_language(&langid!("en-GB"))
        .expect("should fall back from en-GB to en");

    // Missing required argument should produce formatting errors and return None.
    assert_eq!(localizer.localize("welcome", None), None);

    // fr has only a partial resource plan, so it can activate the ready
    // resources it has and fall back for anything missing.
    localizer
        .select_language(&langid!("fr"))
        .expect("partial locale should activate available resources");
    assert_eq!(
        localizer.localize("hello", None),
        Some("Bonjour depuis le fichier de base FR".to_string())
    );

    // it is declared as supported but has no resources.
    let it_err = localizer
        .select_language(&langid!("it"))
        .expect_err("missing files should fail");
    assert!(matches!(it_err, LocalizationError::LanguageNotSupported(_)));

    // de is not in the supported list.
    let de_err = localizer
        .select_language(&langid!("de"))
        .expect_err("unsupported language should fail");
    assert!(matches!(de_err, LocalizationError::LanguageNotSupported(_)));
}

#[test]
fn embedded_localizer_keeps_previous_bundle_when_selection_fails() {
    let localizer = EmbeddedLocalizer::<TestAssets>::new(&MODULE_DATA);

    localizer
        .select_language(&langid!("en"))
        .expect("en should load successfully");
    assert_eq!(
        localizer.localize("ui-title", None),
        Some("UI Title".to_string())
    );

    localizer
        .select_language(&langid!("fr"))
        .expect("partial locale should switch successfully");
    assert_eq!(
        localizer.localize("hello", None),
        Some("Bonjour depuis le fichier de base FR".to_string())
    );
    assert_eq!(
        localizer.localize("ui-title", None),
        None,
        "partial locales should not keep resources from the previous active locale"
    );
}

#[test]
fn embedded_module_data_and_factory_work() {
    let module = EmbeddedI18nModule::<TestAssets>::new(&MODULE_DATA);
    assert_eq!(module.data().name, "test-module");
    let localizer = module.create_localizer();
    assert_eq!(localizer.localize("hello", None), None);
}

#[test]
fn embedded_localizer_exercises_namespaced_parse_and_missing_namespace_paths() {
    let localizer = EmbeddedLocalizer::<NamespaceErrorAssets>::new(&NS_ERROR_MODULE_DATA);

    let parse_err = localizer
        .select_language(&langid!("ab"))
        .expect_err("invalid namespaced FTL should fail");
    assert!(matches!(
        parse_err,
        LocalizationError::LanguageNotSupported(_)
    ));

    localizer
        .select_language(&langid!("ef"))
        .expect("base-only locale should activate its exact resource plan");
    assert_eq!(
        localizer.localize("hello", None),
        Some("Hello from EF".to_string())
    );
}

#[test]
fn embedded_localizer_ignores_noncanonical_base_files() {
    let localizer = EmbeddedLocalizer::<StrayBaseFileAssets>::new(&STRAY_BASE_FILE_MODULE_DATA);

    localizer
        .select_language(&langid!("en"))
        .expect("noncanonical base files should not block namespaced readiness");
    assert_eq!(
        localizer.localize("hello", None),
        Some("Hello from stray-base fixture".to_string())
    );
}

#[test]
fn embedded_localizer_loads_nested_namespace_files() {
    let localizer = EmbeddedLocalizer::<NestedNamespaceAssets>::new(&NESTED_NAMESPACE_MODULE_DATA);

    localizer
        .select_language(&langid!("en"))
        .expect("nested namespace file should make the locale ready");
    assert_eq!(
        localizer.localize("nested-title", None),
        Some("Nested UI Button".to_string())
    );
}

#[test]
fn embedded_localizer_rejects_bundle_add_errors_and_preserves_previous_bundle() {
    let localizer = EmbeddedLocalizer::<BundleAddErrorAssets>::new(&BUNDLE_ADD_ERROR_MODULE_DATA);

    localizer
        .select_language(&langid!("en"))
        .expect("en should load successfully");
    assert_eq!(
        localizer.localize("hello", None),
        Some("Hello from bundle-add fixture".to_string())
    );

    let err = localizer
        .select_language(&langid!("fr"))
        .expect_err("duplicate ids across bundle resources should fail selection");
    let bundle_error = match err {
        LocalizationError::IoError(io_error) => io_error
            .get_ref()
            .and_then(|error| error.downcast_ref::<BundleBuildError>())
            .cloned()
            .expect("bundle build diagnostics should be preserved inside the io error"),
        other => panic!("expected io-backed bundle build error, got {other:?}"),
    };

    assert_eq!(bundle_error.module_name(), "bundle-add-error-module");
    assert_eq!(bundle_error.language(), &langid!("fr"));
    assert!(
        bundle_error
            .to_string()
            .contains("failed to build a Fluent bundle for module 'bundle-add-error-module'")
    );
    assert!(
        bundle_error
            .diagnostics()
            .iter()
            .any(|message| message.contains("hello")),
        "bundle build diagnostics should mention the duplicate message"
    );
    assert_eq!(
        localizer.localize("hello", None),
        Some("Hello from bundle-add fixture".to_string()),
        "failed switches should keep the last ready locale active"
    );
}
