//! Inventory collection functionality for CLI commands.

use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Expected key information from inventory.
#[derive(Serialize)]
pub struct ExpectedKey {
    pub key: String,
    pub variables: Vec<String>,
    /// The Rust source file where this key is defined.
    pub source_file: Option<String>,
    /// The line number in the Rust source file.
    pub source_line: Option<u32>,
}

/// The inventory data output.
#[derive(Serialize)]
pub struct InventoryData {
    pub expected_keys: Vec<ExpectedKey>,
}

/// Intermediate metadata for a key during collection.
#[derive(Default)]
struct KeyMeta {
    variables: HashSet<String>,
    source_file: Option<String>,
    source_line: Option<u32>,
}

/// Collects inventory data for a crate and writes it to `inventory.json`.
///
/// This function is used by the es-fluent CLI to collect expected FTL keys
/// and their variables from inventory-registered types.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to collect inventory for (e.g., "my-crate")
///
/// # Panics
///
/// Panics if serialization or file writing fails.
pub fn write_inventory_for_crate(crate_name: &str) {
    let crate_ident = crate_name.replace('-', "_");

    // Collect all registered type infos for this crate
    let type_infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .into_iter()
        .filter(|info| {
            info.module_path == crate_ident
                || info.module_path.starts_with(&format!("{}::", crate_ident))
        })
        .collect();

    // Build a map of expected keys with their metadata
    let mut keys_map: HashMap<String, KeyMeta> = HashMap::new();
    for info in &type_infos {
        for variant in &info.variants {
            let key = variant.ftl_key.clone();
            let vars: HashSet<String> = variant.args.iter().cloned().collect();
            let entry = keys_map.entry(key).or_insert_with(|| KeyMeta {
                variables: HashSet::new(),
                source_file: info.file_path.clone(),
                source_line: Some(variant.line),
            });
            entry.variables.extend(vars);
            // Keep the first source location we encounter
        }
    }

    // Convert to output format
    let expected_keys: Vec<ExpectedKey> = keys_map
        .into_iter()
        .map(|(key, meta)| ExpectedKey {
            key,
            variables: meta.variables.into_iter().collect(),
            source_file: meta.source_file,
            source_line: meta.source_line,
        })
        .collect();

    let data = InventoryData { expected_keys };

    // Write inventory data to file
    let json = serde_json::to_string(&data).expect("Failed to serialize inventory data");

    let metadata_dir = std::path::Path::new("metadata").join(crate_name);
    std::fs::create_dir_all(&metadata_dir).expect("Failed to create metadata directory");

    std::fs::write(metadata_dir.join("inventory.json"), json)
        .expect("Failed to write inventory file");
}
