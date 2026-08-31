use super::*;
use std::fs;
use std::path::Path;

const I18N_CONFIG: &str = "fallback_language = \"en\"\nassets_dir = \"i18n\"\n";

/// Compute blake3 hash of all .rs files in a source directory, plus the i18n.toml file.
///
/// Used for staleness detection - saving a file without modifications
/// won't change the hash, avoiding unnecessary rebuilds.
///
/// The `i18n_toml_path` parameter includes the i18n.toml configuration file
/// in the hash, so changes to settings like `fluent_feature` trigger rebuilds.
pub fn compute_content_hash(src_dir: &Path, i18n_toml_path: Option<&Path>) -> String {
    use blake3::Hasher;

    let mut hasher = Hasher::new();
    hash_rs_sources(&mut hasher, src_dir, &[]);

    // Include i18n.toml if provided and exists
    if let Some(toml_path) = i18n_toml_path
        && toml_path.is_file()
    {
        hash_optional_file(&mut hasher, "i18n.toml", toml_path);
    }

    hasher.finalize().to_hex().to_string()
}

mod caches;
mod cargo_inputs;
mod content_hash;
mod crate_inputs;
