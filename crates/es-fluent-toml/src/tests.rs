use super::*;
use std::fs;
use tempfile::TempDir;

use crate::test_utils::with_manifest_env;

#[test]
fn test_read_from_path_success() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    let config_content = r#"
fallback_language = "en"
assets_dir = "i18n"
"#;

    fs::write(&config_path, config_content).unwrap();

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
fn test_available_locale_names_preserve_raw_directory_names() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_dir = temp_dir.path();
    let assets = manifest_dir.join("i18n");
    fs::create_dir(&assets).unwrap();
    fs::create_dir(assets.join("en-us")).unwrap();
    fs::create_dir(assets.join("fr")).unwrap();

    let config = I18nConfig {
        fallback_language: "en-us".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let locales = config
        .available_locale_names_from_base(Some(manifest_dir))
        .unwrap();
    assert_eq!(locales, vec!["en-us", "fr"]);

    let canonical_languages = config
        .available_languages_from_base(Some(manifest_dir))
        .unwrap()
        .into_iter()
        .map(|lang| lang.to_string())
        .collect::<Vec<_>>();
    assert_eq!(canonical_languages, vec!["en-US", "fr"]);
}

#[test]
fn test_fluent_feature_single_string() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    let config_content = r#"
fallback_language = "en"
assets_dir = "i18n"
fluent_feature = "fluent"
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = I18nConfig::read_from_path(&config_path).unwrap();
    let features = config.fluent_feature.unwrap().as_vec();
    assert_eq!(features, vec!["fluent"]);
}

#[test]
fn test_fluent_feature_array() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    let config_content = r#"
fallback_language = "en"
assets_dir = "i18n"
fluent_feature = ["fluent", "i18n"]
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = I18nConfig::read_from_path(&config_path).unwrap();
    let features = config.fluent_feature.unwrap().as_vec();
    assert_eq!(features, vec!["fluent", "i18n"]);
}

#[test]
fn test_fluent_feature_none() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("i18n.toml");

    let config_content = r#"
fallback_language = "en"
assets_dir = "i18n"
"#;

    fs::write(&config_path, config_content).unwrap();

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
    fs::write(
        temp_dir.path().join("i18n.toml"),
        "fallback_language = \"en-US\"\nassets_dir = \"locales\"\n",
    )
    .unwrap();

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
fn test_available_languages_rejects_variant_language_directory() {
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

    let err = config
        .available_languages_from_base(Some(temp_dir.path()))
        .expect_err("variant language directory must fail");
    assert!(matches!(
        err,
        I18nConfigError::UnsupportedLanguageIdentifier { name, reason }
            if name == "en-oxendict" && reason == "variants are not supported"
    ));
}

#[test]
fn test_fallback_language_identifier_rejects_variants() {
    let config = I18nConfig {
        fallback_language: "en-oxendict".to_string(),
        assets_dir: PathBuf::from("i18n"),
        fluent_feature: None,
        namespaces: None,
    };

    let err = config
        .fallback_language_identifier()
        .expect_err("variant fallback should fail");
    assert!(matches!(
        err,
        I18nConfigError::UnsupportedLanguageIdentifier { name, reason }
            if name == "en-oxendict" && reason == "variants are not supported"
    ));
}
