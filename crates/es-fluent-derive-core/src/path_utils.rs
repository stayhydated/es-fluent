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
