//! Common path utilities for the es-fluent ecosystem.

use crate::error::{EsFluentError, EsFluentResult};
use std::path::{Path, PathBuf};

/// Get all locale directories from an assets directory.
///
/// Returns a sorted list of locale directory names.
pub fn get_all_locales(assets_dir: &Path) -> EsFluentResult<Vec<String>> {
    let mut locales = Vec::new();

    if !assets_dir.exists() {
        return Ok(locales);
    }

    let entries = std::fs::read_dir(assets_dir)?;

    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            locales.push(name.to_string());
        }
    }

    locales.sort();
    Ok(locales)
}

/// Create metadata directory and return the path.
///
/// Creates a `metadata/{crate_name}` directory structure.
pub fn create_metadata_dir(crate_name: &str) -> EsFluentResult<PathBuf> {
    let metadata_dir = Path::new("metadata").join(crate_name);
    std::fs::create_dir_all(&metadata_dir)?;
    Ok(metadata_dir)
}

/// Get the path to the result.json file for a given crate.
///
/// Returns `metadata/{crate_name}/result.json` without creating directories.
pub fn get_metadata_result_path<T: AsRef<std::path::Path>>(
    temp_dir: T,
    crate_name: &str,
) -> PathBuf {
    temp_dir
        .as_ref()
        .join("metadata")
        .join(crate_name)
        .join("result.json")
}

/// Get the path to the inventory.json file for a given crate.
///
/// Returns `metadata/{crate_name}/inventory.json` without creating directories.
pub fn get_metadata_inventory_path<T: AsRef<std::path::Path>>(
    temp_dir: T,
    crate_name: &str,
) -> PathBuf {
    temp_dir
        .as_ref()
        .join("metadata")
        .join(crate_name)
        .join("inventory.json")
}

/// Get the path to the .es-fluent temporary directory for a workspace.
///
/// Returns `{workspace_root}/.es-fluent`.
pub fn get_es_fluent_temp_dir<T: AsRef<std::path::Path>>(workspace_root: T) -> PathBuf {
    workspace_root.as_ref().join(".es-fluent")
}

/// Write result data to metadata directory.
///
/// Creates the metadata directory if needed and writes the result to `result.json`.
pub fn write_metadata_result<T: serde::Serialize>(
    crate_name: &str,
    result: &T,
) -> EsFluentResult<()> {
    let metadata_dir = create_metadata_dir(crate_name)?;
    let json = serde_json::to_string(result)?;
    std::fs::write(metadata_dir.join("result.json"), json)?;
    Ok(())
}

/// Parse a directory entry as a language identifier.
///
/// Returns `Ok(None)` if the entry is not a directory.
pub fn parse_language_entry(
    entry: std::fs::DirEntry,
) -> EsFluentResult<Option<unic_langid::LanguageIdentifier>> {
    if !entry.file_type()?.is_dir() {
        return Ok(None);
    }

    let raw_name = entry.file_name();
    let name = raw_name.into_string().map_err(|raw| {
        EsFluentError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Assets directory contains a non UTF-8 entry: {:?}", raw),
        ))
    })?;

    let lang = name
        .parse::<unic_langid::LanguageIdentifier>()
        .map_err(|e| {
            EsFluentError::invalid_language_identifier(&name, format!("Parse error: {}", e))
        })?;

    ensure_supported_language_identifier(&lang, &name)?;
    Ok(Some(lang))
}

/// Ensure the language identifier is supported (no variants).
fn ensure_supported_language_identifier(
    lang: &unic_langid::LanguageIdentifier,
    original: &str,
) -> EsFluentResult<()> {
    if lang.variants().next().is_some() {
        return Err(EsFluentError::invalid_language_identifier(
            original,
            "variants are not supported",
        ));
    }
    Ok(())
}

/// Validate that assets directory exists and is a directory.
pub fn validate_assets_dir(assets_dir: &Path) -> EsFluentResult<()> {
    if !assets_dir.exists() {
        return Err(EsFluentError::assets_not_found(assets_dir));
    }

    if !assets_dir.is_dir() {
        return Err(EsFluentError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Assets path '{}' is not a directory", assets_dir.display()),
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::{LazyLock, Mutex};
    use tempfile::tempdir;

    static CWD_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn with_temp_cwd<T>(f: impl FnOnce(&Path) -> T) -> T {
        let _guard = CWD_LOCK.lock().expect("lock poisoned");
        let original = std::env::current_dir().expect("current dir");
        let temp = tempdir().expect("tempdir");
        std::env::set_current_dir(temp.path()).expect("set cwd");
        let result = f(temp.path());
        std::env::set_current_dir(original).expect("restore cwd");
        result
    }

    fn read_dir_entry_by_name(parent: &Path, name: &str) -> std::fs::DirEntry {
        std::fs::read_dir(parent)
            .expect("read_dir")
            .filter_map(|entry| entry.ok())
            .find(|entry| entry.file_name() == OsString::from(name))
            .expect("entry not found")
    }

    #[test]
    fn get_all_locales_returns_sorted_directories_only() {
        let temp = tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("fr")).expect("create fr");
        std::fs::create_dir_all(temp.path().join("en-US")).expect("create en-US");
        std::fs::write(temp.path().join("README.txt"), "ignore me").expect("write file");

        let locales = get_all_locales(temp.path()).expect("get locales");
        assert_eq!(locales, vec!["en-US".to_string(), "fr".to_string()]);
    }

    #[test]
    fn get_all_locales_returns_empty_for_missing_dir() {
        let temp = tempdir().expect("tempdir");
        let missing = temp.path().join("missing");
        let locales = get_all_locales(&missing).expect("get locales");
        assert!(locales.is_empty());
    }

    #[test]
    fn create_metadata_dir_and_write_metadata_result_use_workspace_relative_paths() {
        with_temp_cwd(|cwd| {
            let metadata_dir = create_metadata_dir("my_crate").expect("create metadata dir");
            assert_eq!(metadata_dir, Path::new("metadata").join("my_crate"));
            assert!(cwd.join(&metadata_dir).is_dir());

            write_metadata_result("my_crate", &serde_json::json!({ "changed": true }))
                .expect("write result");
            let result_path = cwd.join("metadata").join("my_crate").join("result.json");
            let content = std::fs::read_to_string(result_path).expect("read result");
            assert_eq!(content, r#"{"changed":true}"#);
        });
    }

    #[test]
    fn metadata_path_helpers_build_expected_paths() {
        let base = Path::new("/tmp/example");
        assert_eq!(
            get_metadata_result_path(base, "crate-x"),
            Path::new("/tmp/example/metadata/crate-x/result.json")
        );
        assert_eq!(
            get_metadata_inventory_path(base, "crate-x"),
            Path::new("/tmp/example/metadata/crate-x/inventory.json")
        );
        assert_eq!(
            get_es_fluent_temp_dir(base),
            Path::new("/tmp/example/.es-fluent")
        );
    }

    #[test]
    fn parse_language_entry_handles_non_directory_and_valid_directory() {
        let temp = tempdir().expect("tempdir");
        std::fs::write(temp.path().join("file.txt"), "not a directory").expect("write");
        std::fs::create_dir_all(temp.path().join("en-US")).expect("create lang");

        let file_entry = read_dir_entry_by_name(temp.path(), "file.txt");
        assert_eq!(parse_language_entry(file_entry).expect("parse file"), None);

        let dir_entry = read_dir_entry_by_name(temp.path(), "en-US");
        let parsed = parse_language_entry(dir_entry)
            .expect("parse dir")
            .expect("language id");
        assert_eq!(
            parsed,
            "en-US"
                .parse::<unic_langid::LanguageIdentifier>()
                .expect("language")
        );
    }

    #[test]
    fn parse_language_entry_rejects_invalid_or_unsupported_identifiers() {
        let temp = tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("not-a-lang!")).expect("create invalid");
        std::fs::create_dir_all(temp.path().join("de-DE-1901")).expect("create variant");

        let invalid_entry = read_dir_entry_by_name(temp.path(), "not-a-lang!");
        let invalid_err = parse_language_entry(invalid_entry).expect_err("should fail");
        assert!(matches!(
            invalid_err,
            EsFluentError::InvalidLanguageIdentifier { .. }
        ));

        let variant_entry = read_dir_entry_by_name(temp.path(), "de-DE-1901");
        let variant_err = parse_language_entry(variant_entry).expect_err("should fail");
        assert!(matches!(
            variant_err,
            EsFluentError::InvalidLanguageIdentifier { .. }
        ));
    }

    #[test]
    fn validate_assets_dir_checks_missing_file_and_directory() {
        let temp = tempdir().expect("tempdir");
        let missing = temp.path().join("missing");
        let file = temp.path().join("file.txt");
        let dir = temp.path().join("assets");
        std::fs::write(&file, "x").expect("write");
        std::fs::create_dir_all(&dir).expect("mkdir");

        let missing_err = validate_assets_dir(&missing).expect_err("missing should fail");
        assert!(matches!(missing_err, EsFluentError::AssetsNotFound { .. }));

        let file_err = validate_assets_dir(&file).expect_err("file should fail");
        assert!(matches!(file_err, EsFluentError::IoError(_)));

        validate_assets_dir(&dir).expect("directory should validate");
    }
}
