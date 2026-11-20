use super::*;
use crate::I18nModuleDescriptor as _;
use crate::asset_localization::ModuleData;
use crate::localization::{
    BundleBuildError, FluentArgumentMap, I18nModule, LocalizationError, Localizer, ModuleLocalizer,
};
use es_fluent_shared::registry::StaticFluentMessageKey;
use fluent_bundle::FluentValue;
use std::sync::LazyLock;
use unic_langid::{LanguageIdentifier, langid};

fn static_entry(value: &'static str) -> StaticFluentMessageKey {
    crate::__macro::static_message_key(
        "test-domain",
        crate::__macro::static_domain("test-domain"),
        crate::__macro::static_entry_id(value),
    )
}

fn leak(value: &str) -> &'static str {
    Box::leak(value.to_owned().into_boxed_str())
}

/// Builds a fixture's embedded table from the planner `define_i18n_module!`
/// runs at build time, so the table matches what the macro generates.
fn fixture_languages(
    fixture: &str,
    files: &[(&str, &'static [u8])],
) -> &'static [EmbeddedLanguage] {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let (_, _, plans) =
        es_fluent_shared::resource::ResourcePlan::sparse_from_assets("test-domain", &root)
            .expect("fixture resource plan")
            .into_parts();
    let languages = plans
        .into_iter()
        .map(|(language, specs)| {
            let resources = specs
                .into_iter()
                .map(|spec| {
                    let path = format!("{language}/{}", spec.locale_relative_path);
                    let contents = files
                        .iter()
                        .find(|(file, _)| *file == path)
                        .map(|(_, contents)| *contents)
                        .unwrap_or_else(|| panic!("planned fixture file {path} is not embedded"));
                    EmbeddedResource::new(
                        leak(spec.key.as_str()),
                        leak(spec.locale_relative_path.as_str()),
                        spec.required,
                        contents,
                    )
                })
                .collect::<Vec<_>>();
            EmbeddedLanguage {
                language,
                resources: Box::leak(resources.into_boxed_slice()),
            }
        })
        .collect::<Vec<_>>();
    Box::leak(languages.into_boxed_slice())
}

macro_rules! test_embedded_languages {
    ($name:ident, $fixture:literal, [$($path:literal),* $(,)?]) => {
        static $name: LazyLock<&'static [EmbeddedLanguage]> = LazyLock::new(|| {
            fixture_languages(
                $fixture,
                &[$((
                    $path,
                    include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/tests/fixtures/",
                        $fixture,
                        "/",
                        $path
                    )) as &[u8],
                )),*],
            )
        });
    };
}

test_embedded_languages!(
    TEST_LANGUAGES,
    "embedded_i18n",
    [
        "en-GB/test-domain.ftl",
        "en/test-domain.ftl",
        "en/test-domain/ui.ftl",
        "fr/test-domain.ftl",
    ]
);

test_embedded_languages!(
    NAMESPACE_ERROR_LANGUAGES,
    "embedded_i18n_ns_errors",
    [
        "ab/test-domain.ftl",
        "ab/test-domain/ui.ftl",
        "ef/test-domain.ftl",
    ]
);

test_embedded_languages!(
    STRAY_BASE_FILE_LANGUAGES,
    "embedded_i18n_stray_base_file",
    ["en/test-domain.ftl", "en/test-domain/ui.ftl"]
);

test_embedded_languages!(
    NESTED_NAMESPACE_LANGUAGES,
    "embedded_i18n_nested",
    ["en/test-domain/ui/button.ftl"]
);

test_embedded_languages!(
    BUNDLE_ADD_ERROR_LANGUAGES,
    "embedded_i18n_bundle_add_error",
    [
        "en/test-domain.ftl",
        "en/test-domain/errors.ftl",
        "en/test-domain/ui.ftl",
        "fr/test-domain.ftl",
        "fr/test-domain/errors.ftl",
        "fr/test-domain/ui.ftl",
    ]
);

test_embedded_languages!(
    PARTIAL_FALLBACK_LANGUAGES,
    "embedded_i18n_partial_fallback",
    ["en-US/test-domain/ui.ftl", "en/test-domain/ui.ftl"]
);

static SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[
    langid!("en"),
    langid!("en-GB"),
    langid!("fr"),
    langid!("it"),
];
static NAMESPACES: &[&str] = &["ui"];
static MODULE_DATA: ModuleData = ModuleData {
    name: "test-module",
    owner: crate::__macro::static_domain("test-domain"),
    supported_languages: SUPPORTED_LANGUAGES,
    domains: &[crate::ModuleDomain {
        domain: crate::__macro::static_domain("test-domain"),
        namespaces: NAMESPACES,
    }],
};
static BASE_FILE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static BASE_FILE_MODULE_DATA: ModuleData = ModuleData {
    name: "base-file-module",
    owner: crate::__macro::static_domain("test-domain"),
    supported_languages: BASE_FILE_SUPPORTED_LANGUAGES,
    domains: &[crate::ModuleDomain {
        domain: crate::__macro::static_domain("test-domain"),
        namespaces: &[],
    }],
};
static NS_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("ab"), langid!("ef")];
static NS_ERROR_MODULE_DATA: ModuleData = ModuleData {
    name: "ns-error-module",
    owner: crate::__macro::static_domain("test-domain"),
    supported_languages: NS_ERROR_SUPPORTED_LANGUAGES,
    domains: &[crate::ModuleDomain {
        domain: crate::__macro::static_domain("test-domain"),
        namespaces: NAMESPACES,
    }],
};
static STRAY_BASE_FILE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static STRAY_BASE_FILE_MODULE_DATA: ModuleData = ModuleData {
    name: "stray-base-file-module",
    owner: crate::__macro::static_domain("test-domain"),
    supported_languages: STRAY_BASE_FILE_SUPPORTED_LANGUAGES,
    domains: &[crate::ModuleDomain {
        domain: crate::__macro::static_domain("test-domain"),
        namespaces: NAMESPACES,
    }],
};
static NESTED_NAMESPACE_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
static NESTED_NAMESPACE_MODULE_DATA: ModuleData = ModuleData {
    name: "nested-namespace-module",
    owner: crate::__macro::static_domain("test-domain"),
    supported_languages: NESTED_NAMESPACE_SUPPORTED_LANGUAGES,
    domains: &[crate::ModuleDomain {
        domain: crate::__macro::static_domain("test-domain"),
        namespaces: &["ui/button"],
    }],
};
static BUNDLE_ADD_ERROR_SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
    &[langid!("en"), langid!("fr")];
static BUNDLE_ADD_ERROR_MODULE_DATA: ModuleData = ModuleData {
    name: "bundle-add-error-module",
    owner: crate::__macro::static_domain("test-domain"),
    supported_languages: BUNDLE_ADD_ERROR_SUPPORTED_LANGUAGES,
    domains: &[crate::ModuleDomain {
        domain: crate::__macro::static_domain("test-domain"),
        namespaces: &["ui", "errors"],
    }],
};
static PARTIAL_FALLBACK_SUPPORTED_LANGUAGES: &[LanguageIdentifier] =
    &[langid!("en-US"), langid!("en")];
static PARTIAL_FALLBACK_MODULE_DATA: ModuleData = ModuleData {
    name: "partial-fallback-module",
    owner: crate::__macro::static_domain("test-domain"),
    supported_languages: PARTIAL_FALLBACK_SUPPORTED_LANGUAGES,
    domains: &[crate::ModuleDomain {
        domain: crate::__macro::static_domain("test-domain"),
        namespaces: NAMESPACES,
    }],
};
fn localizer(
    data: &'static ModuleData,
    languages: &'static [EmbeddedLanguage],
    lang: LanguageIdentifier,
) -> Result<Box<dyn Localizer>, LocalizationError> {
    I18nModule::create_localizer(&EmbeddedI18nModule::new(data, languages), &lang)
}

#[test]
fn embedded_localizer_loads_base_file_modules_without_namespaces() {
    let localizer = localizer(&BASE_FILE_MODULE_DATA, *TEST_LANGUAGES, langid!("en"))
        .expect("base file should make the locale ready");

    assert_eq!(
        localizer.localize(static_entry("base-only"), None),
        Some("Hello main".to_string())
    );
}

#[test]
fn embedded_localizer_uses_fallback_and_formats_with_args() {
    let localizer = localizer(&MODULE_DATA, *TEST_LANGUAGES, langid!("en-US"))
        .expect("fallback to en should work");

    let mut args = FluentArgumentMap::default();
    args.set("name", FluentValue::from("Mark"));
    let welcome = localizer.localize(static_entry("welcome"), Some(&args));
    assert!(
        welcome
            .as_deref()
            .is_some_and(|value| value.contains("Welcome") && value.contains("Mark"))
    );
    assert_eq!(
        localizer.localize(static_entry("base-only"), None),
        Some("Hello main".to_string())
    );
    assert_eq!(
        localizer.localize(static_entry("ui-title"), None),
        Some("UI Title".to_string())
    );
}

#[test]
fn embedded_localizer_fallback_bundles_keep_their_own_locale() {
    let module = EmbeddedI18nModule::new(&MODULE_DATA, *TEST_LANGUAGES);
    let localizer = ModuleLocalizer::new(&MODULE_DATA, &langid!("en-US"), |candidate| {
        module.load_language(candidate)
    })
    .expect("fallback to en should work");

    let bundles = localizer
        .bundles("test-domain")
        .expect("bundles should be built");
    // Fluent picks plural rules from a bundle's first locale when it is built,
    // so a fallback bundle formats with the locale its resources were written in.
    assert_eq!(bundles[0].locales, vec![langid!("en")]);
}

#[test]
fn embedded_localizer_falls_back_per_message() {
    let localizer = localizer(
        &PARTIAL_FALLBACK_MODULE_DATA,
        *PARTIAL_FALLBACK_LANGUAGES,
        langid!("en-US"),
    )
    .expect("partial locale should fall back to en for missing messages");

    assert_eq!(
        localizer.localize(static_entry("hello"), None),
        Some("Hello from en-US".to_string())
    );
    assert_eq!(
        localizer.localize(static_entry("ui-title"), None),
        Some("Shared UI Title".to_string())
    );
}

#[test]
fn embedded_localizer_exercises_fallback_and_missing_resource_paths() {
    // en-GB does not have a ready canonical namespace resource, so creation
    // falls back to en.
    let en_gb = localizer(&MODULE_DATA, *TEST_LANGUAGES, langid!("en-GB"))
        .expect("should fall back from en-GB to en");
    // Missing required argument should produce formatting errors and return None.
    assert_eq!(en_gb.localize(static_entry("welcome"), None), None);

    // fr has only a partial resource plan, so it activates the ready resources
    // it has without borrowing other locales.
    let fr = localizer(&MODULE_DATA, *TEST_LANGUAGES, langid!("fr"))
        .expect("partial locale should activate available resources");
    assert_eq!(
        fr.localize(static_entry("hello"), None),
        Some("Bonjour depuis le fichier de base FR".to_string())
    );
    assert_eq!(fr.localize(static_entry("ui-title"), None), None);

    // it is declared as supported but has no resources.
    let it_err = localizer(&MODULE_DATA, *TEST_LANGUAGES, langid!("it"))
        .err()
        .expect("missing files should fail");
    assert!(matches!(it_err, LocalizationError::LanguageNotSupported(_)));

    // de is not in the supported list.
    let de_err = localizer(&MODULE_DATA, *TEST_LANGUAGES, langid!("de"))
        .err()
        .expect("unsupported language should fail");
    assert!(matches!(de_err, LocalizationError::LanguageNotSupported(_)));
}

#[test]
fn embedded_module_data_and_resource_plans() {
    let module = EmbeddedI18nModule::new(&MODULE_DATA, *TEST_LANGUAGES);
    assert_eq!(module.data().name, "test-module");

    let registration = &module as &dyn crate::I18nModuleRegistration;
    let plan = registration
        .resource_plan_for_language(&langid!("en"))
        .expect("embedded registration should expose its locale resource plan");
    assert_eq!(plan.len(), 2);
    assert!(
        plan.iter()
            .all(|resource| resource.key.domain() == "test-domain")
    );
    assert_eq!(
        registration.resource_plan_for_language(&langid!("de")),
        None
    );
}

#[test]
fn embedded_resources_are_parsed_once() {
    let resource = &TEST_LANGUAGES[0].resources[0];
    let first = resource.resource().expect("fixture should parse");
    let second = resource.resource().expect("fixture should parse");
    assert!(std::sync::Arc::ptr_eq(&first, &second));
}

#[test]
fn embedded_localizer_exercises_namespaced_parse_and_missing_namespace_paths() {
    let parse_err = localizer(
        &NS_ERROR_MODULE_DATA,
        *NAMESPACE_ERROR_LANGUAGES,
        langid!("ab"),
    )
    .err()
    .expect("invalid namespaced FTL should fail");
    assert!(matches!(
        parse_err,
        LocalizationError::LanguageNotSupported(_)
    ));

    let ef = localizer(
        &NS_ERROR_MODULE_DATA,
        *NAMESPACE_ERROR_LANGUAGES,
        langid!("ef"),
    )
    .expect("base-only locale should activate its exact resource plan");
    assert_eq!(
        ef.localize(static_entry("hello"), None),
        Some("Hello from EF".to_string())
    );
}

#[test]
fn embedded_localizer_ignores_noncanonical_base_files() {
    let localizer = localizer(
        &STRAY_BASE_FILE_MODULE_DATA,
        *STRAY_BASE_FILE_LANGUAGES,
        langid!("en"),
    )
    .expect("noncanonical base files should not block namespaced readiness");
    assert_eq!(
        localizer.localize(static_entry("hello"), None),
        Some("Hello from stray-base fixture".to_string())
    );
}

#[test]
fn embedded_localizer_loads_nested_namespace_files() {
    let localizer = localizer(
        &NESTED_NAMESPACE_MODULE_DATA,
        *NESTED_NAMESPACE_LANGUAGES,
        langid!("en"),
    )
    .expect("nested namespace file should make the locale ready");
    assert_eq!(
        localizer.localize(static_entry("nested-title"), None),
        Some("Nested UI Button".to_string())
    );
}

#[test]
fn embedded_localizer_rejects_bundle_add_errors() {
    let en = localizer(
        &BUNDLE_ADD_ERROR_MODULE_DATA,
        *BUNDLE_ADD_ERROR_LANGUAGES,
        langid!("en"),
    )
    .expect("en should load successfully");
    assert_eq!(
        en.localize(static_entry("hello"), None),
        Some("Hello from bundle-add fixture".to_string())
    );

    let err = localizer(
        &BUNDLE_ADD_ERROR_MODULE_DATA,
        *BUNDLE_ADD_ERROR_LANGUAGES,
        langid!("fr"),
    )
    .err()
    .expect("duplicate ids across bundle resources should fail");
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
}
