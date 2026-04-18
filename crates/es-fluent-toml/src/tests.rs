use super::*;
use fs_err as fs;
use tempfile::TempDir;

use crate::test_utils::with_manifest_env;

fn string_value(value: &str) -> toml::Value {
    toml::Value::String(value.to_string())
}

fn table(
    entries: impl IntoIterator<Item = (&'static str, toml::Value)>,
) -> toml::map::Map<String, toml::Value> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn write_toml(path: &Path, value: &toml::Value) {
    fs::write(
        path,
        toml::to_string(value).expect("serialize TOML fixture"),
    )
    .unwrap();
}

fn config_document(
    fallback_language: &str,
    assets_dir: &str,
    fluent_feature: Option<toml::Value>,
    namespaces: Option<Vec<&str>>,
) -> toml::Value {
    let mut config = table([
        ("fallback_language", string_value(fallback_language)),
        ("assets_dir", string_value(assets_dir)),
    ]);
    if let Some(fluent_feature) = fluent_feature {
        config.insert("fluent_feature".to_string(), fluent_feature);
    }
    if let Some(namespaces) = namespaces {
        config.insert(
            "namespaces".to_string(),
            toml::Value::Array(namespaces.into_iter().map(string_value).collect()),
        );
    }
    toml::Value::Table(config)
}

#[test]
fn test_read_from_path_success() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    write_toml(&config_path, &config_document("en", "i18n", None, None));

    let result = I18nConfig::read_from_path(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.fallback_language, "en");
    assert_eq!(config.assets_dir, PathBuf::from("i18n"));
}

#[test]
fn test_read_from_path_file_not_found() {
    let non_existent_path = Path::new("/non/existent/path/i18n.toml");
    let result = I18nConfig::read_from_path(non_existent_path);
    assert!(matches!(result, Err(I18nConfigError::NotFound)));
}

#[test]
fn test_read_from_path_invalid_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    let invalid_config = r#"
fallback_language = "en"
[invalid_section]
assets_dir = "i18n"
"#;

    fs::write(&config_path, invalid_config).unwrap();

    let result = I18nConfig::read_from_path(&config_path);
    assert!(matches!(result, Err(I18nConfigError::ParseError(_))));
}

#[test]
fn test_read_from_path_rejects_noncanonical_fallback_language() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    write_toml(&config_path, &config_document("en-us", "i18n", None, None));

    let result = I18nConfig::read_from_path(&config_path);
    assert!(matches!(
        result,
        Err(I18nConfigError::NonCanonicalFallbackLanguageIdentifier { name, canonical })
            if name == "en-us" && canonical == "en-US"
    ));
}

#[test]
fn test_assets_dir_path() {
    let config = I18nConfig {
        fallback_language: "en-US".to_string(),
        assets_dir: PathBuf::from("locales"),
        fluent_feature: None,
        namespaces: None,
    };

    assert_eq!(config.assets_dir_path(), PathBuf::from("locales"));
}

#[test]
fn test_fallback_language_id() {
    let config = I18nConfig {
        fallback_language: "en-US".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    assert_eq!(config.fallback_language_id(), "en-US");
}

#[test]
fn test_fallback_language_identifier_success() {
    let config = I18nConfig {
        fallback_language: "en-US".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let lang = config.fallback_language_identifier().unwrap();

    assert_eq!(lang.to_string(), "en-US");
}

#[test]
fn test_fallback_language_identifier_invalid() {
    let config = I18nConfig {
        fallback_language: "invalid-lang!".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let result = config.fallback_language_identifier();

    assert!(matches!(
        result,
        Err(I18nConfigError::InvalidFallbackLanguageIdentifier { name, .. })
            if name == "invalid-lang!"
    ));
}

#[test]
fn test_available_languages_collects_directories() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_dir = temp_dir.path();
    let assets = manifest_dir.join("i18n");
    fs::create_dir(&assets).unwrap();
    fs::create_dir(assets.join("en")).unwrap();
    fs::create_dir(assets.join("en-US")).unwrap();
    fs::create_dir(assets.join("fr")).unwrap();
    fs::create_dir(assets.join("zh-Hans")).unwrap();
    fs::write(assets.join("README.txt"), "ignored file").unwrap();

    let config = I18nConfig {
        fallback_language: "en".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let languages = config
        .available_languages_from_base(Some(manifest_dir))
        .unwrap();

    let mut codes: Vec<String> = languages.into_iter().map(|lang| lang.to_string()).collect();
    codes.sort();

    assert_eq!(codes, vec!["en", "en-US", "fr", "zh-Hans"]);
}

#[test]
fn test_available_languages_allows_language_only() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_dir = temp_dir.path();
    let assets = manifest_dir.join("i18n");
    fs::create_dir(&assets).unwrap();
    fs::create_dir(assets.join("en")).unwrap();

    let config = I18nConfig {
        fallback_language: "en".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let languages = config
        .available_languages_from_base(Some(manifest_dir))
        .unwrap();
    let codes: Vec<String> = languages.into_iter().map(|lang| lang.to_string()).collect();

    assert_eq!(codes, vec!["en"]);
}

#[test]
fn test_available_locale_names_reject_noncanonical_directory_names() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_dir = temp_dir.path();
    let assets = manifest_dir.join("i18n");
    fs::create_dir(&assets).unwrap();
    fs::create_dir(assets.join("en-us")).unwrap();
    fs::create_dir(assets.join("fr")).unwrap();

    let config = I18nConfig {
        fallback_language: "en-US".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let locale_err = config
        .available_locale_names_from_base(Some(manifest_dir))
        .expect_err("noncanonical locale directories should fail");
    assert!(matches!(
        locale_err,
        I18nConfigError::NonCanonicalLanguageIdentifier { name, canonical }
            if name == "en-us" && canonical == "en-US"
    ));

    let language_err = config
        .available_languages_from_base(Some(manifest_dir))
        .expect_err("noncanonical locale directories should fail");
    assert!(matches!(
        language_err,
        I18nConfigError::NonCanonicalLanguageIdentifier { name, canonical }
            if name == "en-us" && canonical == "en-US"
    ));
}

#[test]
fn test_resolved_layout_helpers_delegate_to_underlying_config() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join("i18n/en-US")).unwrap();
    fs::create_dir_all(temp_dir.path().join("i18n/fr")).unwrap();
    write_toml(
        &temp_dir.path().join("i18n.toml"),
        &config_document(
            "en-US",
            "i18n",
            Some(toml::Value::Array(vec![
                string_value("fluent"),
                string_value("i18n"),
            ])),
            Some(vec!["ui", "errors"]),
        ),
    );

    let layout = ResolvedI18nLayout::from_manifest_dir(temp_dir.path()).unwrap();
    assert_eq!(layout.fallback_language(), "en-US");
    assert_eq!(layout.locale_dir("fr"), temp_dir.path().join("i18n/fr"));
    assert_eq!(layout.fluent_features(), vec!["fluent", "i18n"]);
    assert_eq!(
        layout.allowed_namespaces(),
        Some(&["ui".to_string(), "errors".to_string()][..])
    );

    let mut locales = layout.available_locale_names().unwrap();
    locales.sort();
    assert_eq!(locales, vec!["en-US", "fr"]);

    let mut languages: Vec<_> = layout
        .available_languages()
        .unwrap()
        .into_iter()
        .map(|lang| lang.to_string())
        .collect();
    languages.sort();
    assert_eq!(languages, vec!["en-US", "fr"]);
}

#[test]
fn test_available_languages_and_locale_names_use_manifest_env() {
    let temp_dir = TempDir::new().unwrap();
    let assets_dir = temp_dir.path().join("i18n");
    fs::create_dir_all(assets_dir.join("en")).unwrap();
    fs::create_dir_all(assets_dir.join("fr")).unwrap();

    let config = I18nConfig {
        fallback_language: "en".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    with_manifest_env(Some(temp_dir.path()), || {
        let mut locale_names = config.available_locale_names().unwrap();
        locale_names.sort();
        assert_eq!(locale_names, vec!["en", "fr"]);

        let mut languages: Vec<_> = config
            .available_languages()
            .unwrap()
            .into_iter()
            .map(|lang| lang.to_string())
            .collect();
        languages.sort();
        assert_eq!(languages, vec!["en", "fr"]);
    });
}

#[test]
fn test_fluent_feature_single_string() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    write_toml(
        &config_path,
        &config_document("en", "i18n", Some(string_value("fluent")), None),
    );

    let config = I18nConfig::read_from_path(&config_path).unwrap();
    let features = config.fluent_feature.unwrap().as_vec();
    assert_eq!(features, vec!["fluent"]);
}

#[test]
fn test_fluent_feature_array() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    write_toml(
        &config_path,
        &config_document(
            "en",
            "i18n",
            Some(toml::Value::Array(vec![
                string_value("fluent"),
                string_value("i18n"),
            ])),
            None,
        ),
    );

    let config = I18nConfig::read_from_path(&config_path).unwrap();
    let features = config.fluent_feature.unwrap().as_vec();
    assert_eq!(features, vec!["fluent", "i18n"]);
}

#[test]
fn test_fluent_feature_none() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    write_toml(&config_path, &config_document("en", "i18n", None, None));

    let config = I18nConfig::read_from_path(&config_path).unwrap();
    assert!(config.fluent_feature.is_none());
}

#[test]
fn test_fluent_feature_is_empty_variants() {
    assert!(FluentFeature::Single(String::new()).is_empty());
    assert!(!FluentFeature::Single("fluent".to_string()).is_empty());
    assert!(FluentFeature::Multiple(Vec::new()).is_empty());
    assert!(!FluentFeature::Multiple(vec!["fluent".to_string()]).is_empty());
}

#[test]
#[serial_test::serial(manifest)]
fn test_available_languages_uses_manifest_env_when_base_not_provided() {
    let temp_dir = TempDir::new().unwrap();
    let assets = temp_dir.path().join("i18n");
    fs::create_dir(&assets).unwrap();
    fs::create_dir(assets.join("en")).unwrap();

    let config = I18nConfig {
        fallback_language: "en".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let languages = with_manifest_env(Some(temp_dir.path()), || config.available_languages())
        .expect("available languages");
    assert_eq!(
        languages
            .into_iter()
            .map(|lang| lang.to_string())
            .collect::<Vec<_>>(),
        vec!["en"]
    );
}

#[test]
#[serial_test::serial(manifest)]
fn test_validate_assets_dir_reports_missing_and_non_directory() {
    let temp_dir = TempDir::new().unwrap();
    let config = I18nConfig {
        fallback_language: "en".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let missing = with_manifest_env(Some(temp_dir.path()), || config.validate_assets_dir());
    assert!(matches!(
        missing,
        Err(I18nConfigError::ReadError(err)) if err.kind() == std::io::ErrorKind::NotFound
    ));

    fs::write(temp_dir.path().join("i18n"), "not a directory").unwrap();
    let not_directory = with_manifest_env(Some(temp_dir.path()), || config.validate_assets_dir());
    assert!(matches!(
        not_directory,
        Err(I18nConfigError::ReadError(err)) if err.kind() == std::io::ErrorKind::InvalidInput
    ));
}

#[test]
fn test_manifest_dir_helper_methods() {
    let temp_dir = TempDir::new().unwrap();
    write_toml(
        &temp_dir.path().join("i18n.toml"),
        &config_document("en-US", "locales", None, None),
    );

    let config = I18nConfig::from_manifest_dir(temp_dir.path()).expect("config");
    assert_eq!(config.fallback_language, "en-US");
    assert_eq!(config.assets_dir, PathBuf::from("locales"));

    let assets = I18nConfig::assets_dir_from_manifest_dir(temp_dir.path()).expect("assets");
    assert_eq!(assets, temp_dir.path().join("locales"));

    let output = I18nConfig::output_dir_from_manifest_dir(temp_dir.path()).expect("output");
    assert_eq!(output, temp_dir.path().join("locales/en-US"));
}

#[test]
fn test_available_languages_rejects_invalid_language_directory() {
    let temp_dir = TempDir::new().unwrap();
    let assets = temp_dir.path().join("i18n");
    fs::create_dir(&assets).unwrap();
    fs::create_dir(assets.join("invalid-lang!")).unwrap();

    let config = I18nConfig {
        fallback_language: "en".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let err = config
        .available_languages_from_base(Some(temp_dir.path()))
        .expect_err("invalid language directory must fail");
    assert!(matches!(
        err,
        I18nConfigError::InvalidLanguageIdentifier { name, .. } if name == "invalid-lang!"
    ));
}

#[test]
fn test_available_languages_accepts_variant_language_directory() {
    let temp_dir = TempDir::new().unwrap();
    let assets = temp_dir.path().join("i18n");
    fs::create_dir(&assets).unwrap();
    fs::create_dir(assets.join("en-oxendict")).unwrap();

    let config = I18nConfig {
        fallback_language: "en".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let languages = config
        .available_languages_from_base(Some(temp_dir.path()))
        .expect("variant language directory should be accepted");
    assert_eq!(
        languages,
        vec![
            "en-oxendict"
                .parse::<unic_langid::LanguageIdentifier>()
                .expect("language")
        ]
    );
}

#[test]
fn test_collect_language_entries_propagates_directory_iteration_errors() {
    let err = collect_language_entries([Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "boom",
    ))])
    .expect_err("directory iteration errors should not be dropped");

    assert!(matches!(
        err,
        I18nConfigError::ReadError(inner) if inner.kind() == std::io::ErrorKind::PermissionDenied
    ));
}

#[test]
fn test_fallback_language_identifier_accepts_variants() {
    let config = I18nConfig {
        fallback_language: "en-oxendict".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let language = config
        .fallback_language_identifier()
        .expect("variant fallback language should parse");
    assert_eq!(
        language,
        "en-oxendict"
            .parse::<unic_langid::LanguageIdentifier>()
            .expect("language")
    );
}
