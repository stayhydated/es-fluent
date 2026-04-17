//! Common path utilities for the es-fluent ecosystem.

use crate::error::{EsFluentError, EsFluentResult};
use std::path::Path;

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
    let canonical = lang.to_string();
    if canonical != name {
        return Err(EsFluentError::invalid_language_identifier(
            &name,
            format!(
                "Locale directory must use canonical BCP-47 casing '{}'",
                canonical
            ),
        ));
    }

    Ok(Some(lang))
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
    use tempfile::tempdir;

    fn read_dir_entry_by_name(parent: &Path, name: &str) -> std::fs::DirEntry {
        std::fs::read_dir(parent)
            .expect("read_dir")
            .filter_map(|entry| entry.ok())
            .find(|entry| entry.file_name() == OsString::from(name))
            .expect("entry not found")
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
    fn parse_language_entry_rejects_invalid_identifiers_and_accepts_variants() {
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
        let variant_lang = parse_language_entry(variant_entry)
            .expect("variant locale should parse")
            .expect("language id");
        assert_eq!(
            variant_lang,
            "de-DE-1901"
                .parse::<unic_langid::LanguageIdentifier>()
                .expect("language")
        );
    }

    #[test]
    fn parse_language_entry_rejects_noncanonical_locale_casing() {
        let temp = tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("en-us")).expect("create noncanonical");

        let entry = read_dir_entry_by_name(temp.path(), "en-us");
        let err = parse_language_entry(entry).expect_err("noncanonical locale should fail");
        assert!(matches!(
            err,
            EsFluentError::InvalidLanguageIdentifier { .. }
        ));
        assert!(err.to_string().contains("en-US"));
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
