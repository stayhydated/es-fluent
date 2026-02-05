use anyhow::{Context as _, Result};
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;

/// Expected key information from inventory (deserialized from temp crate output).
#[derive(Deserialize)]
struct ExpectedKey {
    key: String,
    variables: Vec<String>,
    /// The Rust source file where this key is defined.
    source_file: Option<String>,
    /// The line number in the Rust source file.
    source_line: Option<u32>,
}

/// Runtime info about an expected key with its variables and source location.
#[derive(Clone)]
pub(crate) struct KeyInfo {
    pub(crate) variables: HashSet<String>,
    pub(crate) source_file: Option<String>,
    pub(crate) source_line: Option<u32>,
}

/// The inventory data output from the temp crate.
#[derive(Deserialize)]
struct InventoryData {
    expected_keys: Vec<ExpectedKey>,
}

/// Read inventory data from the generated inventory.json file.
pub(crate) fn read_inventory_file(
    temp_dir: &std::path::Path,
    crate_name: &str,
) -> Result<IndexMap<String, KeyInfo>> {
    let inventory_path = es_fluent_derive_core::get_metadata_inventory_path(temp_dir, crate_name);
    let json_str = fs::read_to_string(&inventory_path)
        .with_context(|| format!("Failed to read {}", inventory_path.display()))?;

    let data: InventoryData =
        serde_json::from_str(&json_str).context("Failed to parse inventory JSON")?;

    // Convert to IndexMap with KeyInfo for richer metadata
    let mut expected_keys = IndexMap::new();
    for key_info in data.expected_keys {
        expected_keys.insert(
            key_info.key,
            KeyInfo {
                variables: key_info.variables.into_iter().collect(),
                source_file: key_info.source_file,
                source_line: key_info.source_line,
            },
        );
    }

    Ok(expected_keys)
}
